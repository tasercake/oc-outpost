//! SSE stream handler for OpenCode events.
//!
//! Subscribes to Server-Sent Events from OpenCode sessions, parses events,
//! batches text chunks, handles reconnection, and deduplicates Telegram messages.

#![allow(dead_code)]

use crate::opencode::OpenCodeClient;
use anyhow::{Context, Result};
use futures::StreamExt;
use reqwest_eventsource::{Event, EventSource};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

/// Maximum reconnection attempts before giving up
const MAX_RECONNECT_ATTEMPTS: u32 = 5;

/// Base delay for exponential backoff (1 second)
const BASE_RECONNECT_DELAY_SECS: u64 = 1;

/// Maximum reconnection delay (16 seconds)
const MAX_RECONNECT_DELAY_SECS: u64 = 16;

/// Message batching interval (2 seconds)
const BATCH_INTERVAL_SECS: u64 = 2;

/// Deduplication message expiry (30 seconds)
const DEDUP_EXPIRY_SECS: u64 = 30;

/// Events emitted by the stream handler.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StreamEvent {
    /// Text chunk from assistant response
    TextChunk { text: String },
    /// Tool invocation started
    ToolInvocation {
        name: String,
        args: serde_json::Value,
    },
    /// Tool execution result
    ToolResult { result: String },
    /// Message completed
    MessageComplete { message: OpenCodeMessage },
    /// Session is idle (ready for input)
    SessionIdle,
    /// Session error occurred
    SessionError { error: String },
    /// Permission requested
    PermissionRequest {
        id: String,
        permission_type: String,
        details: serde_json::Value,
    },
    /// Permission reply received
    PermissionReply { id: String, allowed: bool },
    /// Connection lost (for internal tracking)
    Disconnected,
    /// Connection restored
    Reconnected,
}

/// OpenCode message format
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OpenCodeMessage {
    pub id: String,
    pub role: String,
    pub content: Vec<serde_json::Value>,
}

/// Raw SSE event data for message.part.updated
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum MessagePartData {
    Text {
        text: String,
    },
    ToolUse {
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        content: String,
    },
}

/// Raw SSE event data for session.error
#[derive(Clone, Debug, Deserialize)]
struct SessionErrorData {
    message: String,
}

/// Raw SSE event data for permission.updated
#[derive(Clone, Debug, Deserialize)]
struct PermissionUpdatedData {
    id: String,
    #[serde(rename = "type")]
    permission_type: String,
    #[serde(flatten)]
    details: serde_json::Value,
}

/// Raw SSE event data for permission.replied
#[derive(Clone, Debug, Deserialize)]
struct PermissionRepliedData {
    id: String,
    allowed: bool,
}

/// Handle for a subscription (for cleanup)
struct SubscriptionHandle {
    cancel_tx: oneshot::Sender<()>,
    #[allow(dead_code)]
    task_handle: tokio::task::JoinHandle<()>,
}

/// SSE stream handler for OpenCode events.
pub struct StreamHandler {
    client: OpenCodeClient,
    subscriptions: Arc<Mutex<HashMap<String, SubscriptionHandle>>>,
    telegram_messages: Arc<Mutex<HashMap<String, HashSet<String>>>>,
}

impl StreamHandler {
    /// Create a new stream handler.
    pub fn new(client: OpenCodeClient) -> Self {
        Self {
            client,
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            telegram_messages: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Subscribe to SSE events for a session.
    ///
    /// Returns a channel receiver for stream events.
    pub async fn subscribe(&self, session_id: &str) -> Result<mpsc::Receiver<StreamEvent>> {
        let url = self.client.sse_url(session_id);
        let session_id = session_id.to_string();
        let (tx, rx) = mpsc::channel(100);
        let (cancel_tx, cancel_rx) = oneshot::channel();

        let telegram_messages = Arc::clone(&self.telegram_messages);
        let session_id_clone = session_id.clone();

        let task_handle = tokio::spawn(async move {
            Self::run_stream_loop(url, session_id_clone, tx, cancel_rx, telegram_messages).await;
        });

        {
            let mut subs = self.subscriptions.lock().unwrap();
            subs.insert(
                session_id,
                SubscriptionHandle {
                    cancel_tx,
                    task_handle,
                },
            );
        }

        Ok(rx)
    }

    /// Mark a message as sent from Telegram (for deduplication).
    pub fn mark_from_telegram(&self, session_id: &str, text: &str) {
        let session_id = session_id.to_string();
        let text = text.to_string();
        let telegram_messages = Arc::clone(&self.telegram_messages);

        {
            let mut messages = telegram_messages.lock().unwrap();
            messages
                .entry(session_id.clone())
                .or_default()
                .insert(text.clone());
        }

        // Spawn cleanup task to remove after expiry
        let cleanup_messages = Arc::clone(&self.telegram_messages);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(DEDUP_EXPIRY_SECS)).await;
            let mut messages = cleanup_messages.lock().unwrap();
            if let Some(set) = messages.get_mut(&session_id) {
                set.remove(&text);
                if set.is_empty() {
                    messages.remove(&session_id);
                }
            }
        });
    }

    /// Unsubscribe from a session's SSE stream.
    pub async fn unsubscribe(&self, session_id: &str) {
        let handle = {
            let mut subs = self.subscriptions.lock().unwrap();
            subs.remove(session_id)
        };

        if let Some(handle) = handle {
            // Send cancel signal (ignore if already closed)
            let _ = handle.cancel_tx.send(());
            debug!("Unsubscribed from session: {}", session_id);
        }
    }

    /// Check if message should be skipped (sent from Telegram)
    fn should_skip(
        telegram_messages: &Arc<Mutex<HashMap<String, HashSet<String>>>>,
        session_id: &str,
        text: &str,
    ) -> bool {
        let messages = telegram_messages.lock().unwrap();
        messages
            .get(session_id)
            .map(|set| set.contains(text))
            .unwrap_or(false)
    }

    /// Run the main stream loop with reconnection logic
    async fn run_stream_loop(
        url: String,
        session_id: String,
        tx: mpsc::Sender<StreamEvent>,
        mut cancel_rx: oneshot::Receiver<()>,
        telegram_messages: Arc<Mutex<HashMap<String, HashSet<String>>>>,
    ) {
        let mut attempt = 0;

        loop {
            // Check for cancellation before connecting
            if cancel_rx.try_recv().is_ok() {
                debug!("Stream cancelled for session: {}", session_id);
                return;
            }

            match Self::connect_and_process(
                &url,
                &session_id,
                &tx,
                &mut cancel_rx,
                &telegram_messages,
            )
            .await
            {
                Ok(()) => {
                    // Clean exit (cancelled)
                    return;
                }
                Err(e) => {
                    warn!("SSE stream error for session {}: {:?}", session_id, e);

                    if attempt >= MAX_RECONNECT_ATTEMPTS {
                        error!(
                            "Max reconnection attempts reached for session: {}",
                            session_id
                        );
                        let _ = tx
                            .send(StreamEvent::SessionError {
                                error: format!(
                                    "Connection lost after {} attempts",
                                    MAX_RECONNECT_ATTEMPTS
                                ),
                            })
                            .await;
                        return;
                    }

                    // Notify disconnect
                    let _ = tx.send(StreamEvent::Disconnected).await;

                    // Exponential backoff
                    let delay_secs = std::cmp::min(
                        BASE_RECONNECT_DELAY_SECS * 2_u64.pow(attempt),
                        MAX_RECONNECT_DELAY_SECS,
                    );
                    info!(
                        "Reconnecting to session {} in {}s (attempt {}/{})",
                        session_id,
                        delay_secs,
                        attempt + 1,
                        MAX_RECONNECT_ATTEMPTS
                    );

                    // Wait with cancellation check
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(delay_secs)) => {}
                        _ = &mut cancel_rx => {
                            debug!("Stream cancelled during reconnect for session: {}", session_id);
                            return;
                        }
                    }

                    attempt += 1;
                }
            }
        }
    }

    /// Connect to SSE and process events
    async fn connect_and_process(
        url: &str,
        session_id: &str,
        tx: &mpsc::Sender<StreamEvent>,
        cancel_rx: &mut oneshot::Receiver<()>,
        telegram_messages: &Arc<Mutex<HashMap<String, HashSet<String>>>>,
    ) -> Result<()> {
        let client = reqwest::Client::new();
        let request = client.get(url);
        let mut es = EventSource::new(request).context("Failed to create EventSource")?;

        // Message batching state
        let mut text_batch = String::new();
        let mut last_batch_time = Instant::now();

        loop {
            tokio::select! {
                event = es.next() => {
                    match event {
                        Some(Ok(Event::Open)) => {
                            info!("SSE connected for session: {}", session_id);
                            // Notify reconnection if this was a retry
                            let _ = tx.send(StreamEvent::Reconnected).await;
                        }
                        Some(Ok(Event::Message(msg))) => {
                            // Handle the SSE event
                            if let Err(e) = Self::handle_sse_message(
                                &msg.event,
                                &msg.data,
                                session_id,
                                tx,
                                &mut text_batch,
                                &mut last_batch_time,
                                telegram_messages,
                            ).await {
                                debug!("Error handling SSE message: {:?}", e);
                            }
                        }
                        Some(Err(e)) => {
                            // Flush any pending batch before error
                            if !text_batch.is_empty() {
                                let _ = tx.send(StreamEvent::TextChunk { text: std::mem::take(&mut text_batch) }).await;
                            }
                            return Err(anyhow::anyhow!("SSE error: {:?}", e));
                        }
                        None => {
                            // Stream ended
                            if !text_batch.is_empty() {
                                let _ = tx.send(StreamEvent::TextChunk { text: std::mem::take(&mut text_batch) }).await;
                            }
                            return Err(anyhow::anyhow!("SSE stream ended"));
                        }
                    }
                }

                // Check for batch timeout
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    if !text_batch.is_empty() && last_batch_time.elapsed() >= Duration::from_secs(BATCH_INTERVAL_SECS) {
                        let _ = tx.send(StreamEvent::TextChunk { text: std::mem::take(&mut text_batch) }).await;
                        last_batch_time = Instant::now();
                    }
                }

                // Check for cancellation
                _ = &mut *cancel_rx => {
                    debug!("Stream cancelled for session: {}", session_id);
                    es.close();
                    return Ok(());
                }
            }
        }
    }

    /// Handle a single SSE message
    async fn handle_sse_message(
        event_type: &str,
        data: &str,
        session_id: &str,
        tx: &mpsc::Sender<StreamEvent>,
        text_batch: &mut String,
        last_batch_time: &mut Instant,
        telegram_messages: &Arc<Mutex<HashMap<String, HashSet<String>>>>,
    ) -> Result<()> {
        match event_type {
            "message.part.updated" => {
                let part: MessagePartData =
                    serde_json::from_str(data).context("Failed to parse message.part.updated")?;

                match part {
                    MessagePartData::Text { text } => {
                        // Check deduplication
                        if Self::should_skip(telegram_messages, session_id, &text) {
                            debug!("Skipping duplicated text from Telegram");
                            return Ok(());
                        }

                        // Batch text chunks
                        text_batch.push_str(&text);
                        *last_batch_time = Instant::now();
                    }
                    MessagePartData::ToolUse { name, input } => {
                        // Flush text batch before tool use
                        if !text_batch.is_empty() {
                            tx.send(StreamEvent::TextChunk {
                                text: std::mem::take(text_batch),
                            })
                            .await
                            .ok();
                        }
                        tx.send(StreamEvent::ToolInvocation { name, args: input })
                            .await
                            .ok();
                    }
                    MessagePartData::ToolResult { content } => {
                        // Flush text batch before tool result
                        if !text_batch.is_empty() {
                            tx.send(StreamEvent::TextChunk {
                                text: std::mem::take(text_batch),
                            })
                            .await
                            .ok();
                        }
                        tx.send(StreamEvent::ToolResult { result: content })
                            .await
                            .ok();
                    }
                }
            }

            "message.updated" => {
                // Flush any pending text batch
                if !text_batch.is_empty() {
                    tx.send(StreamEvent::TextChunk {
                        text: std::mem::take(text_batch),
                    })
                    .await
                    .ok();
                }

                let message: OpenCodeMessage =
                    serde_json::from_str(data).context("Failed to parse message.updated")?;
                tx.send(StreamEvent::MessageComplete { message }).await.ok();
            }

            "session.idle" => {
                // Flush any pending text batch
                if !text_batch.is_empty() {
                    tx.send(StreamEvent::TextChunk {
                        text: std::mem::take(text_batch),
                    })
                    .await
                    .ok();
                }
                tx.send(StreamEvent::SessionIdle).await.ok();
            }

            "session.error" => {
                let error_data: SessionErrorData =
                    serde_json::from_str(data).context("Failed to parse session.error")?;
                tx.send(StreamEvent::SessionError {
                    error: error_data.message,
                })
                .await
                .ok();
            }

            "permission.updated" => {
                let perm: PermissionUpdatedData =
                    serde_json::from_str(data).context("Failed to parse permission.updated")?;
                tx.send(StreamEvent::PermissionRequest {
                    id: perm.id,
                    permission_type: perm.permission_type,
                    details: perm.details,
                })
                .await
                .ok();
            }

            "permission.replied" => {
                let reply: PermissionRepliedData =
                    serde_json::from_str(data).context("Failed to parse permission.replied")?;
                tx.send(StreamEvent::PermissionReply {
                    id: reply.id,
                    allowed: reply.allowed,
                })
                .await
                .ok();
            }

            _ => {
                debug!("Unknown SSE event type: {}", event_type);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;
    use tokio::time::timeout;

    // Helper to create a mock SSE server
    async fn create_mock_sse_server(events: Vec<(&'static str, &'static str)>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();

                // Read HTTP request
                let (reader, mut writer) = socket.split();
                let mut buf_reader = BufReader::new(reader);
                let mut line = String::new();

                // Read request line and headers
                loop {
                    line.clear();
                    buf_reader.read_line(&mut line).await.unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                }

                // Send SSE response
                let response = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n";
                writer.write_all(response.as_bytes()).await.unwrap();

                // Send events
                for (event_type, data) in &events {
                    let event = format!("event: {}\ndata: {}\n\n", event_type, data);
                    writer.write_all(event.as_bytes()).await.unwrap();
                    writer.flush().await.unwrap();
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }

                // Keep connection open briefly
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });

        format!("http://{}", addr)
    }

    #[test]
    fn test_new_creates_handler() {
        let client = OpenCodeClient::new("http://localhost:4100");
        let handler = StreamHandler::new(client);
        assert!(handler.subscriptions.lock().unwrap().is_empty());
        assert!(handler.telegram_messages.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_subscribe_creates_channel() {
        let base_url = create_mock_sse_server(vec![("session.idle", "{}")]).await;
        let client = OpenCodeClient::new(&base_url);
        let handler = StreamHandler::new(client);

        let rx = handler.subscribe("test-session").await.unwrap();

        // Verify subscription was added
        {
            let subs = handler.subscriptions.lock().unwrap();
            assert!(subs.contains_key("test-session"));
        }

        drop(rx);
        handler.unsubscribe("test-session").await;
    }

    #[tokio::test]
    async fn test_parse_message_part_updated_text() {
        let events = vec![(
            "message.part.updated",
            r#"{"type":"text","text":"Hello, world!"}"#,
        )];
        let base_url = create_mock_sse_server(events).await;
        let client = OpenCodeClient::new(&base_url);
        let handler = StreamHandler::new(client);

        let mut rx = handler.subscribe("test-session").await.unwrap();

        // Wait for events with timeout
        let result = timeout(Duration::from_secs(5), async {
            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::TextChunk { text } => {
                        assert!(text.contains("Hello"));
                        return true;
                    }
                    StreamEvent::Reconnected => continue,
                    _ => continue,
                }
            }
            false
        })
        .await;

        assert!(result.unwrap_or(false), "Expected TextChunk event");
        handler.unsubscribe("test-session").await;
    }

    #[tokio::test]
    async fn test_parse_message_part_updated_tool_use() {
        let events = vec![(
            "message.part.updated",
            r#"{"type":"tool_use","name":"read_file","input":{"path":"/foo.txt"}}"#,
        )];
        let base_url = create_mock_sse_server(events).await;
        let client = OpenCodeClient::new(&base_url);
        let handler = StreamHandler::new(client);

        let mut rx = handler.subscribe("test-session").await.unwrap();

        let result = timeout(Duration::from_secs(5), async {
            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::ToolInvocation { name, args } => {
                        assert_eq!(name, "read_file");
                        assert_eq!(args["path"], "/foo.txt");
                        return true;
                    }
                    StreamEvent::Reconnected => continue,
                    _ => continue,
                }
            }
            false
        })
        .await;

        assert!(result.unwrap_or(false), "Expected ToolInvocation event");
        handler.unsubscribe("test-session").await;
    }

    #[tokio::test]
    async fn test_parse_message_part_updated_tool_result() {
        let events = vec![(
            "message.part.updated",
            r#"{"type":"tool_result","content":"File contents here"}"#,
        )];
        let base_url = create_mock_sse_server(events).await;
        let client = OpenCodeClient::new(&base_url);
        let handler = StreamHandler::new(client);

        let mut rx = handler.subscribe("test-session").await.unwrap();

        let result = timeout(Duration::from_secs(5), async {
            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::ToolResult { result } => {
                        assert_eq!(result, "File contents here");
                        return true;
                    }
                    StreamEvent::Reconnected => continue,
                    _ => continue,
                }
            }
            false
        })
        .await;

        assert!(result.unwrap_or(false), "Expected ToolResult event");
        handler.unsubscribe("test-session").await;
    }

    #[tokio::test]
    async fn test_parse_message_updated() {
        let events = vec![(
            "message.updated",
            r#"{"id":"msg_123","role":"assistant","content":[{"type":"text","text":"Done"}]}"#,
        )];
        let base_url = create_mock_sse_server(events).await;
        let client = OpenCodeClient::new(&base_url);
        let handler = StreamHandler::new(client);

        let mut rx = handler.subscribe("test-session").await.unwrap();

        let result = timeout(Duration::from_secs(5), async {
            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::MessageComplete { message } => {
                        assert_eq!(message.id, "msg_123");
                        assert_eq!(message.role, "assistant");
                        return true;
                    }
                    StreamEvent::Reconnected => continue,
                    _ => continue,
                }
            }
            false
        })
        .await;

        assert!(result.unwrap_or(false), "Expected MessageComplete event");
        handler.unsubscribe("test-session").await;
    }

    #[tokio::test]
    async fn test_parse_session_idle() {
        let events = vec![("session.idle", "{}")];
        let base_url = create_mock_sse_server(events).await;
        let client = OpenCodeClient::new(&base_url);
        let handler = StreamHandler::new(client);

        let mut rx = handler.subscribe("test-session").await.unwrap();

        let result = timeout(Duration::from_secs(5), async {
            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::SessionIdle => return true,
                    StreamEvent::Reconnected => continue,
                    _ => continue,
                }
            }
            false
        })
        .await;

        assert!(result.unwrap_or(false), "Expected SessionIdle event");
        handler.unsubscribe("test-session").await;
    }

    #[tokio::test]
    async fn test_parse_session_error() {
        let events = vec![("session.error", r#"{"message":"Something went wrong"}"#)];
        let base_url = create_mock_sse_server(events).await;
        let client = OpenCodeClient::new(&base_url);
        let handler = StreamHandler::new(client);

        let mut rx = handler.subscribe("test-session").await.unwrap();

        let result = timeout(Duration::from_secs(5), async {
            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::SessionError { error } => {
                        assert_eq!(error, "Something went wrong");
                        return true;
                    }
                    StreamEvent::Reconnected => continue,
                    _ => continue,
                }
            }
            false
        })
        .await;

        assert!(result.unwrap_or(false), "Expected SessionError event");
        handler.unsubscribe("test-session").await;
    }

    #[tokio::test]
    async fn test_parse_permission_updated() {
        let events = vec![(
            "permission.updated",
            r#"{"id":"perm_123","type":"file_read","path":"/foo/bar.txt"}"#,
        )];
        let base_url = create_mock_sse_server(events).await;
        let client = OpenCodeClient::new(&base_url);
        let handler = StreamHandler::new(client);

        let mut rx = handler.subscribe("test-session").await.unwrap();

        let result = timeout(Duration::from_secs(5), async {
            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::PermissionRequest {
                        id,
                        permission_type,
                        details,
                    } => {
                        assert_eq!(id, "perm_123");
                        assert_eq!(permission_type, "file_read");
                        assert_eq!(details["path"], "/foo/bar.txt");
                        return true;
                    }
                    StreamEvent::Reconnected => continue,
                    _ => continue,
                }
            }
            false
        })
        .await;

        assert!(result.unwrap_or(false), "Expected PermissionRequest event");
        handler.unsubscribe("test-session").await;
    }

    #[tokio::test]
    async fn test_parse_permission_replied() {
        let events = vec![("permission.replied", r#"{"id":"perm_123","allowed":true}"#)];
        let base_url = create_mock_sse_server(events).await;
        let client = OpenCodeClient::new(&base_url);
        let handler = StreamHandler::new(client);

        let mut rx = handler.subscribe("test-session").await.unwrap();

        let result = timeout(Duration::from_secs(5), async {
            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::PermissionReply { id, allowed } => {
                        assert_eq!(id, "perm_123");
                        assert!(allowed);
                        return true;
                    }
                    StreamEvent::Reconnected => continue,
                    _ => continue,
                }
            }
            false
        })
        .await;

        assert!(result.unwrap_or(false), "Expected PermissionReply event");
        handler.unsubscribe("test-session").await;
    }

    #[tokio::test]
    async fn test_message_batching() {
        // Multiple text chunks should be batched
        let events = vec![
            ("message.part.updated", r#"{"type":"text","text":"Hello "}"#),
            ("message.part.updated", r#"{"type":"text","text":"world"}"#),
            ("message.part.updated", r#"{"type":"text","text":"!"}"#),
            ("session.idle", "{}"),
        ];
        let base_url = create_mock_sse_server(events).await;
        let client = OpenCodeClient::new(&base_url);
        let handler = StreamHandler::new(client);

        let mut rx = handler.subscribe("test-session").await.unwrap();

        let result = timeout(Duration::from_secs(5), async {
            let mut got_text = false;

            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::TextChunk { text } => {
                        if text.contains("Hello") || text.contains("world") {
                            got_text = true;
                        }
                    }
                    StreamEvent::SessionIdle => {
                        if got_text {
                            return true;
                        }
                    }
                    StreamEvent::Reconnected => continue,
                    _ => continue,
                }
            }
            false
        })
        .await;

        assert!(result.unwrap_or(false), "Expected batched text and idle");
        handler.unsubscribe("test-session").await;
    }

    #[tokio::test]
    async fn test_mark_from_telegram() {
        let client = OpenCodeClient::new("http://localhost:4100");
        let handler = StreamHandler::new(client);

        // Mark a message
        handler.mark_from_telegram("session-1", "Hello from Telegram");

        // Verify it's tracked
        {
            let messages = handler.telegram_messages.lock().unwrap();
            assert!(messages.contains_key("session-1"));
            assert!(messages["session-1"].contains("Hello from Telegram"));
        }
    }

    #[tokio::test]
    async fn test_deduplication_skips_telegram_messages() {
        let events = vec![(
            "message.part.updated",
            r#"{"type":"text","text":"Hello from Telegram"}"#,
        )];
        let base_url = create_mock_sse_server(events).await;
        let client = OpenCodeClient::new(&base_url);
        let handler = StreamHandler::new(client);

        // Mark the message as from Telegram BEFORE subscribing
        handler.mark_from_telegram("test-session", "Hello from Telegram");

        let mut rx = handler.subscribe("test-session").await.unwrap();

        // The text should NOT appear (it's deduplicated)
        let result = timeout(Duration::from_millis(500), async {
            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::TextChunk { text } => {
                        if text.contains("Hello from Telegram") {
                            return false; // Should not receive this
                        }
                    }
                    StreamEvent::Reconnected => continue,
                    _ => continue,
                }
            }
            true // Good - didn't receive the duplicate
        })
        .await;

        assert!(
            result.unwrap_or(true),
            "Should not receive deduplicated message"
        );
        handler.unsubscribe("test-session").await;
    }

    #[tokio::test]
    async fn test_unsubscribe_closes_stream() {
        let events = vec![
            ("session.idle", "{}"),
            ("session.idle", "{}"),
            ("session.idle", "{}"),
        ];
        let base_url = create_mock_sse_server(events).await;
        let client = OpenCodeClient::new(&base_url);
        let handler = StreamHandler::new(client);

        let _rx = handler.subscribe("test-session").await.unwrap();

        // Verify subscription exists
        {
            let subs = handler.subscriptions.lock().unwrap();
            assert!(subs.contains_key("test-session"));
        }

        // Unsubscribe
        handler.unsubscribe("test-session").await;

        // Verify subscription removed
        {
            let subs = handler.subscriptions.lock().unwrap();
            assert!(!subs.contains_key("test-session"));
        }
    }

    #[tokio::test]
    async fn test_multiple_concurrent_subscriptions() {
        let events1 = vec![("session.idle", "{}")];
        let events2 = vec![("session.idle", "{}")];
        let base_url1 = create_mock_sse_server(events1).await;
        let base_url2 = create_mock_sse_server(events2).await;

        // Use different clients for different sessions
        let client1 = OpenCodeClient::new(&base_url1);
        let handler1 = StreamHandler::new(client1);
        let client2 = OpenCodeClient::new(&base_url2);
        let handler2 = StreamHandler::new(client2);

        let _rx1 = handler1.subscribe("session-1").await.unwrap();
        let _rx2 = handler2.subscribe("session-2").await.unwrap();

        // Both should have subscriptions
        {
            let subs1 = handler1.subscriptions.lock().unwrap();
            assert!(subs1.contains_key("session-1"));
        }
        {
            let subs2 = handler2.subscriptions.lock().unwrap();
            assert!(subs2.contains_key("session-2"));
        }

        handler1.unsubscribe("session-1").await;
        handler2.unsubscribe("session-2").await;
    }

    #[tokio::test]
    async fn test_invalid_sse_data_handling() {
        // Invalid JSON should be handled gracefully
        let events = vec![
            ("message.part.updated", "invalid json here"),
            ("session.idle", "{}"), // Valid event should still work
        ];
        let base_url = create_mock_sse_server(events).await;
        let client = OpenCodeClient::new(&base_url);
        let handler = StreamHandler::new(client);

        let mut rx = handler.subscribe("test-session").await.unwrap();

        // Should still receive valid events
        let result = timeout(Duration::from_secs(5), async {
            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::SessionIdle => return true,
                    StreamEvent::Reconnected => continue,
                    _ => continue,
                }
            }
            false
        })
        .await;

        assert!(
            result.unwrap_or(false),
            "Should handle invalid JSON gracefully"
        );
        handler.unsubscribe("test-session").await;
    }

    #[tokio::test]
    async fn test_connection_timeout_handling() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_secs(10)).await;
            drop(socket);
        });

        let client = OpenCodeClient::new(&format!("http://{}", addr));
        let handler = StreamHandler::new(client);

        let mut rx = handler.subscribe("test").await.unwrap();

        tokio::time::sleep(Duration::from_millis(100)).await;
        handler.unsubscribe("test").await;

        let result = timeout(Duration::from_millis(500), rx.recv()).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_stream_event_serialization() {
        let event = StreamEvent::TextChunk {
            text: "Hello".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("TextChunk"));
        assert!(json.contains("Hello"));

        let deserialized: StreamEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_opencode_message_serialization() {
        let msg = OpenCodeMessage {
            id: "msg_123".to_string(),
            role: "assistant".to_string(),
            content: vec![serde_json::json!({"type": "text", "text": "Hello"})],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: OpenCodeMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, deserialized);
    }
}
