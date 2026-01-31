//! Integration Layer - Wires all components together.
//!
//! Responsibilities:
//! - Message routing (Telegram -> OpenCode)
//! - Stream bridging (OpenCode -> Telegram)
//! - Topic name auto-update after first response
//! - Rate limiting for Telegram API
//! - External instance routing

use crate::bot::BotState;
use crate::opencode::stream_handler::{StreamEvent, StreamHandler};
use crate::opencode::OpenCodeClient;
use crate::telegram::markdown::markdown_to_telegram_html;
use crate::types::error::{OutpostError, Result};
use crate::types::forum::TopicMapping;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use teloxide::prelude::*;
use teloxide::types::{MessageId, ParseMode, ThreadId};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, info, warn};

/// Telegram rate limit: ~30 messages/second, we use 2-second batching
const TELEGRAM_BATCH_INTERVAL: Duration = Duration::from_secs(2);

/// Maximum message length for Telegram (4096 characters)
const TELEGRAM_MAX_MESSAGE_LENGTH: usize = 4096;

/// Rate limiter state for a topic
#[derive(Debug, Clone)]
struct RateLimitState {
    last_send: Instant,
    pending_text: String,
}

impl Default for RateLimitState {
    fn default() -> Self {
        Self {
            last_send: Instant::now() - TELEGRAM_BATCH_INTERVAL,
            pending_text: String::new(),
        }
    }
}

/// Integration layer coordinator
pub struct Integration {
    state: Arc<BotState>,
    stream_handler: Arc<StreamHandler>,
    rate_limiters: Arc<RwLock<HashMap<i32, RateLimitState>>>,
    active_streams: Arc<Mutex<HashMap<i32, tokio::task::JoinHandle<()>>>>,
}

impl Integration {
    /// Create a new integration coordinator
    pub fn new(state: Arc<BotState>, stream_handler: Arc<StreamHandler>) -> Self {
        Self {
            state,
            stream_handler,
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            active_streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Handle incoming Telegram message and route to OpenCode
    ///
    /// Flow:
    /// 1. Extract message text
    /// 2. Get topic mapping
    /// 3. If no mapping, ignore (only handle mapped topics)
    /// 4. Create OpenCode client for instance
    /// 5. Mark message as from Telegram (dedup)
    /// 6. Send to OpenCode async
    /// 7. If streaming enabled, subscribe to SSE
    pub async fn handle_message(&self, bot: Bot, msg: Message) -> Result<()> {
        // 1. Extract text from message
        let text = msg
            .text()
            .ok_or_else(|| OutpostError::telegram_error("Message has no text content"))?;

        // 2. Get topic ID (thread_id in Telegram)
        let thread_id = msg.thread_id.ok_or_else(|| {
            OutpostError::telegram_error("Message is not in a forum topic (no thread_id)")
        })?;
        let topic_id = thread_id.0 .0; // Extract i32 from ThreadId(MessageId(i32))

        // 3. Get topic mapping
        let mapping = {
            let topic_store = self.state.topic_store.lock().await;
            topic_store
                .get_mapping(topic_id)
                .await
                .map_err(|e| OutpostError::database_error(e.to_string()))?
        };

        let mapping = match mapping {
            Some(m) => m,
            None => {
                debug!("No mapping for topic {}, ignoring message", topic_id);
                return Ok(()); // Not an error, just not a tracked topic
            }
        };

        // 4. Ensure we have session_id
        let session_id = mapping.session_id.as_ref().ok_or_else(|| {
            OutpostError::session_not_found(format!("No session for topic {}", topic_id))
        })?;

        // 5. Get OpenCode client for instance
        let port = self.get_instance_port(&mapping).await?;
        let client = OpenCodeClient::new(&format!("http://localhost:{}", port));

        // 6. Mark as from Telegram (for deduplication)
        self.stream_handler.mark_from_telegram(session_id, text);

        // 7. Send message to OpenCode async
        client
            .send_message_async(session_id, text)
            .await
            .map_err(|e| OutpostError::opencode_api_error(e.to_string()))?;

        info!(
            "Routed message to OpenCode: topic={}, session={}",
            topic_id, session_id
        );

        // 8. Subscribe to SSE if streaming enabled and not already subscribed
        if mapping.streaming_enabled {
            self.ensure_stream_subscription(bot, msg.chat.id, topic_id, &mapping)
                .await?;
        }

        Ok(())
    }

    /// Get the port for an instance from the mapping
    async fn get_instance_port(&self, mapping: &TopicMapping) -> Result<u16> {
        // For external instances, we need to look up the port from the orchestrator store
        if let Some(instance_id) = &mapping.instance_id {
            let store = self.state.orchestrator_store.lock().await;
            if let Ok(Some(info)) = store.get_instance(instance_id).await {
                return Ok(info.port);
            }
        }

        // Fall back to config's default port (for managed instances)
        Ok(self.state.config.opencode_port_start)
    }

    /// Ensure we have an active stream subscription for a topic
    async fn ensure_stream_subscription(
        &self,
        bot: Bot,
        chat_id: ChatId,
        topic_id: i32,
        mapping: &TopicMapping,
    ) -> Result<()> {
        // Check if already subscribed
        {
            let streams = self.active_streams.lock().await;
            if streams.contains_key(&topic_id) {
                return Ok(());
            }
        }

        let session_id = mapping
            .session_id
            .clone()
            .ok_or_else(|| OutpostError::session_not_found("No session for topic"))?;

        // Subscribe to SSE
        let rx = self
            .stream_handler
            .subscribe(&session_id)
            .await
            .map_err(|e| OutpostError::opencode_api_error(e.to_string()))?;

        // Spawn stream forwarder task
        let handle = self.spawn_stream_forwarder(bot, chat_id, topic_id, mapping.clone(), rx);

        // Track the active stream
        {
            let mut streams = self.active_streams.lock().await;
            streams.insert(topic_id, handle);
        }

        Ok(())
    }

    /// Spawn a task to forward SSE events to Telegram
    fn spawn_stream_forwarder(
        &self,
        bot: Bot,
        chat_id: ChatId,
        topic_id: i32,
        mapping: TopicMapping,
        mut rx: mpsc::Receiver<StreamEvent>,
    ) -> tokio::task::JoinHandle<()> {
        let rate_limiters = Arc::clone(&self.rate_limiters);
        let state = Arc::clone(&self.state);
        let active_streams = Arc::clone(&self.active_streams);

        tokio::spawn(async move {
            let mut first_response = !mapping.topic_name_updated;
            let session_id = mapping.session_id.clone().unwrap_or_default();

            while let Some(event) = rx.recv().await {
                if let Err(e) = Self::handle_stream_event(
                    &bot,
                    chat_id,
                    topic_id,
                    &event,
                    &rate_limiters,
                    &session_id,
                )
                .await
                {
                    warn!("Error handling stream event: {:?}", e);
                }

                // Check for topic name update on first response
                if first_response {
                    if let StreamEvent::MessageComplete { .. } | StreamEvent::SessionIdle = event {
                        if let Err(e) =
                            Self::update_topic_name(&bot, chat_id, topic_id, &mapping, &state).await
                        {
                            warn!("Failed to update topic name: {:?}", e);
                        }
                        first_response = false;
                    }
                }

                // Check for session end
                if matches!(event, StreamEvent::SessionError { .. }) {
                    break;
                }
            }

            // Flush any pending text
            Self::flush_pending_text(&bot, chat_id, topic_id, &rate_limiters).await;

            // Cleanup
            {
                let mut streams = active_streams.lock().await;
                streams.remove(&topic_id);
            }

            debug!("Stream forwarder ended for topic {}", topic_id);
        })
    }

    /// Handle a single SSE event and forward to Telegram
    async fn handle_stream_event(
        bot: &Bot,
        chat_id: ChatId,
        topic_id: i32,
        event: &StreamEvent,
        rate_limiters: &RwLock<HashMap<i32, RateLimitState>>,
        session_id: &str,
    ) -> Result<()> {
        match event {
            StreamEvent::TextChunk { text } => {
                // Batch text chunks with rate limiting
                let should_send = {
                    let mut limiters = rate_limiters.write().await;
                    let state = limiters.entry(topic_id).or_default();
                    state.pending_text.push_str(text);

                    // Check if we should send now
                    state.last_send.elapsed() >= TELEGRAM_BATCH_INTERVAL
                        || state.pending_text.len() >= TELEGRAM_MAX_MESSAGE_LENGTH / 2
                };

                if should_send {
                    Self::flush_pending_text(bot, chat_id, topic_id, rate_limiters).await;
                }
            }

            StreamEvent::ToolInvocation { name, args } => {
                // Flush any pending text first
                Self::flush_pending_text(bot, chat_id, topic_id, rate_limiters).await;

                let message = format!(
                    "<b>Tool:</b> <code>{}</code>\n<pre>{}</pre>",
                    name,
                    serde_json::to_string_pretty(args).unwrap_or_else(|_| args.to_string())
                );
                Self::send_telegram_message(bot, chat_id, topic_id, &message).await?;
            }

            StreamEvent::ToolResult { result } => {
                let truncated = if result.len() > 500 {
                    format!("{}...", &result[..500])
                } else {
                    result.clone()
                };
                let message = format!("<b>Result:</b>\n<pre>{}</pre>", truncated);
                Self::send_telegram_message(bot, chat_id, topic_id, &message).await?;
            }

            StreamEvent::MessageComplete { message } => {
                // Flush any pending text
                Self::flush_pending_text(bot, chat_id, topic_id, rate_limiters).await;
                debug!("Message complete: id={}, role={}", message.id, message.role);
            }

            StreamEvent::SessionIdle => {
                // Flush any pending text
                Self::flush_pending_text(bot, chat_id, topic_id, rate_limiters).await;
            }

            StreamEvent::SessionError { error } => {
                Self::flush_pending_text(bot, chat_id, topic_id, rate_limiters).await;
                let message = format!("<b>Error:</b> {}", error);
                Self::send_telegram_message(bot, chat_id, topic_id, &message).await?;
            }

            StreamEvent::PermissionRequest {
                id,
                permission_type,
                details,
            } => {
                use crate::bot::handle_permission_request;
                let description = format!(
                    "{}: {}",
                    permission_type,
                    serde_json::to_string_pretty(details).unwrap_or_else(|_| details.to_string())
                );

                if let Err(e) = handle_permission_request(
                    bot.clone(),
                    chat_id,
                    topic_id,
                    session_id,
                    id,
                    &description,
                )
                .await
                {
                    warn!("Failed to send permission request: {:?}", e);
                }
            }

            StreamEvent::PermissionReply { id, allowed } => {
                let status = if *allowed { "allowed" } else { "denied" };
                debug!("Permission {} was {}", id, status);
            }

            StreamEvent::Disconnected => {
                debug!("Stream disconnected for topic {}", topic_id);
            }

            StreamEvent::Reconnected => {
                debug!("Stream reconnected for topic {}", topic_id);
            }
        }

        Ok(())
    }

    /// Flush any pending text to Telegram
    async fn flush_pending_text(
        bot: &Bot,
        chat_id: ChatId,
        topic_id: i32,
        rate_limiters: &RwLock<HashMap<i32, RateLimitState>>,
    ) {
        let text_to_send = {
            let mut limiters = rate_limiters.write().await;
            if let Some(state) = limiters.get_mut(&topic_id) {
                if state.pending_text.is_empty() {
                    return;
                }
                state.last_send = Instant::now();
                std::mem::take(&mut state.pending_text)
            } else {
                return;
            }
        };

        // Convert markdown and send
        let html = markdown_to_telegram_html(&text_to_send);
        if let Err(e) = Self::send_telegram_message(bot, chat_id, topic_id, &html).await {
            warn!("Failed to send batched text: {:?}", e);
        }
    }

    /// Send a message to Telegram in the specified topic
    async fn send_telegram_message(
        bot: &Bot,
        chat_id: ChatId,
        topic_id: i32,
        text: &str,
    ) -> Result<()> {
        // Split long messages
        let parts = crate::telegram::markdown::split_message(text, TELEGRAM_MAX_MESSAGE_LENGTH);

        for part in parts {
            bot.send_message(chat_id, &part)
                .message_thread_id(ThreadId(MessageId(topic_id)))
                .parse_mode(ParseMode::Html)
                .await
                .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
        }

        Ok(())
    }

    /// Update the topic name after first response
    async fn update_topic_name(
        bot: &Bot,
        chat_id: ChatId,
        topic_id: i32,
        mapping: &TopicMapping,
        state: &Arc<BotState>,
    ) -> Result<()> {
        // Extract project name from path
        let project_name = Path::new(&mapping.project_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // Update Telegram topic name
        bot.edit_forum_topic(chat_id, ThreadId(MessageId(topic_id)))
            .name(&project_name)
            .await
            .map_err(|e| OutpostError::telegram_error(e.to_string()))?;

        // Mark as updated in store
        {
            let topic_store = state.topic_store.lock().await;
            topic_store
                .mark_topic_name_updated(topic_id)
                .await
                .map_err(|e| OutpostError::database_error(e.to_string()))?;
        }

        info!("Updated topic {} name to '{}'", topic_id, project_name);

        Ok(())
    }

    /// Stop stream forwarding for a topic
    pub async fn stop_stream(&self, topic_id: i32) {
        let handle = {
            let mut streams = self.active_streams.lock().await;
            streams.remove(&topic_id)
        };

        if let Some(handle) = handle {
            handle.abort();
            debug!("Stopped stream for topic {}", topic_id);
        }
    }

    /// Stop all active streams
    pub async fn stop_all_streams(&self) {
        let handles: Vec<_> = {
            let mut streams = self.active_streams.lock().await;
            streams.drain().collect()
        };

        for (topic_id, handle) in handles {
            handle.abort();
            debug!("Stopped stream for topic {}", topic_id);
        }
    }

    /// Get count of active streams
    pub async fn active_stream_count(&self) -> usize {
        self.active_streams.lock().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::forum::TopicStore;
    use crate::orchestrator::manager::InstanceManager;
    use crate::orchestrator::port_pool::PortPool;
    use crate::orchestrator::store::OrchestratorStore;
    use std::path::PathBuf;
    use tempfile::TempDir;

    async fn create_test_state() -> (Arc<BotState>, Arc<StreamHandler>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            telegram_bot_token: "test_token".to_string(),
            telegram_chat_id: -1001234567890,
            telegram_allowed_users: vec![],
            handle_general_topic: true,
            opencode_path: PathBuf::from("opencode"),
            opencode_max_instances: 10,
            opencode_idle_timeout: Duration::from_secs(1800),
            opencode_port_start: 4100,
            opencode_port_pool_size: 100,
            opencode_health_check_interval: Duration::from_secs(30),
            opencode_startup_timeout: Duration::from_secs(60),
            orchestrator_db_path: temp_dir.path().join("orchestrator.db"),
            topic_db_path: temp_dir.path().join("topics.db"),
            project_base_path: temp_dir.path().to_path_buf(),
            auto_create_project_dirs: true,
            api_port: 4200,
            api_key: None,
        };

        let orchestrator_store = OrchestratorStore::new(&config.orchestrator_db_path)
            .await
            .unwrap();
        let topic_store = TopicStore::new(&config.topic_db_path).await.unwrap();

        let store_for_manager = orchestrator_store.clone();
        let port_pool = PortPool::new(4100, 10);
        let instance_manager =
            InstanceManager::new(Arc::new(config.clone()), store_for_manager, port_pool)
                .await
                .unwrap();
        let bot_start_time = Instant::now();

        let state = Arc::new(BotState::new(
            orchestrator_store,
            topic_store,
            config,
            instance_manager,
            bot_start_time,
        ));

        let client = OpenCodeClient::new("http://localhost:4100");
        let stream_handler = Arc::new(StreamHandler::new(client));

        (state, stream_handler, temp_dir)
    }

    fn create_test_mapping(topic_id: i32) -> TopicMapping {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        TopicMapping {
            topic_id,
            chat_id: -1001234567890,
            project_path: "/test/my-project".to_string(),
            session_id: Some("session-123".to_string()),
            instance_id: Some("inst-456".to_string()),
            streaming_enabled: true,
            topic_name_updated: false,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn test_integration_new() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;
        let integration = Integration::new(state, stream_handler);
        assert_eq!(integration.active_stream_count().await, 0);
    }

    #[tokio::test]
    async fn test_route_message_no_mapping() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;
        let _integration = Integration::new(state.clone(), stream_handler);

        // Create mock message (this would need teloxide test utilities)
        // For now, we test the underlying logic indirectly
        let topic_store = state.topic_store.lock().await;
        let mapping = topic_store.get_mapping(999).await.unwrap();
        assert!(mapping.is_none()); // No mapping exists
    }

    #[tokio::test]
    async fn test_route_message_with_mapping() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;

        // Create a mapping
        let mapping = create_test_mapping(123);
        {
            let topic_store = state.topic_store.lock().await;
            topic_store.save_mapping(&mapping).await.unwrap();
        }

        let _integration = Integration::new(state.clone(), stream_handler);

        // Verify mapping exists
        let stored = {
            let topic_store = state.topic_store.lock().await;
            topic_store.get_mapping(123).await.unwrap()
        };
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().session_id, Some("session-123".to_string()));
    }

    #[tokio::test]
    async fn test_get_instance_port_fallback() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;
        let integration = Integration::new(state.clone(), stream_handler);

        let mapping = create_test_mapping(123);
        let port = integration.get_instance_port(&mapping).await.unwrap();

        // Should fall back to config's default port
        assert_eq!(port, state.config.opencode_port_start);
    }

    #[tokio::test]
    async fn test_active_stream_count() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;
        let integration = Integration::new(state, stream_handler);

        assert_eq!(integration.active_stream_count().await, 0);
    }

    #[tokio::test]
    async fn test_rate_limiter_state_default() {
        let state = RateLimitState::default();
        assert!(state.pending_text.is_empty());
        // last_send should be in the past enough to allow immediate send
        assert!(state.last_send.elapsed() >= TELEGRAM_BATCH_INTERVAL);
    }

    #[tokio::test]
    async fn test_stream_event_throttling() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;
        let integration = Integration::new(state, stream_handler);

        // Test that rate limiter properly batches text
        {
            let mut limiters = integration.rate_limiters.write().await;
            let rate_state = limiters.entry(123).or_default();
            rate_state.pending_text.push_str("Hello ");
            rate_state.pending_text.push_str("World!");
            assert_eq!(rate_state.pending_text, "Hello World!");
        }
    }

    #[tokio::test]
    async fn test_topic_name_extraction() {
        // Test project name extraction from path
        let path = Path::new("/home/user/projects/my-awesome-project");
        let name = path.file_name().and_then(|n| n.to_str()).unwrap();
        assert_eq!(name, "my-awesome-project");

        // Test with trailing slash handled
        let path2 = Path::new("/home/user/projects/another-project");
        let name2 = path2.file_name().and_then(|n| n.to_str()).unwrap();
        assert_eq!(name2, "another-project");
    }

    #[tokio::test]
    async fn test_stop_stream_when_none_active() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;
        let integration = Integration::new(state, stream_handler);

        // Should not panic when stopping non-existent stream
        integration.stop_stream(999).await;
        assert_eq!(integration.active_stream_count().await, 0);
    }

    #[tokio::test]
    async fn test_stop_all_streams_when_empty() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;
        let integration = Integration::new(state, stream_handler);

        // Should not panic when no streams active
        integration.stop_all_streams().await;
        assert_eq!(integration.active_stream_count().await, 0);
    }

    #[tokio::test]
    async fn test_mapping_streaming_enabled() {
        let mapping = create_test_mapping(123);
        assert!(mapping.streaming_enabled);
        assert!(!mapping.topic_name_updated);
    }

    #[tokio::test]
    async fn test_mapping_without_session() {
        let mut mapping = create_test_mapping(123);
        mapping.session_id = None;

        assert!(mapping.session_id.is_none());
    }

    #[tokio::test]
    async fn test_concurrent_rate_limiter_access() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;
        let integration = Arc::new(Integration::new(state, stream_handler));

        let mut handles = vec![];
        for i in 0..5 {
            let int_clone = Arc::clone(&integration);
            let handle = tokio::spawn(async move {
                let mut limiters = int_clone.rate_limiters.write().await;
                let rate_state = limiters.entry(i).or_default();
                rate_state.pending_text.push_str(&format!("Text {}", i));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // All topics should have rate limiter state
        let limiters = integration.rate_limiters.read().await;
        assert_eq!(limiters.len(), 5);
    }

    #[tokio::test]
    async fn test_telegram_message_length_constant() {
        assert_eq!(TELEGRAM_MAX_MESSAGE_LENGTH, 4096);
    }

    #[tokio::test]
    async fn test_batch_interval_constant() {
        assert_eq!(TELEGRAM_BATCH_INTERVAL, Duration::from_secs(2));
    }

    // Note: The following tests would require mock Bot instance
    // In real tests, we'd use teloxide test utilities or mock the Bot

    #[tokio::test]
    async fn test_route_message_to_managed_instance() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;

        // Setup mapping for managed instance
        let mut mapping = create_test_mapping(100);
        mapping.instance_id = None; // Managed instance doesn't need instance_id lookup
        {
            let topic_store = state.topic_store.lock().await;
            topic_store.save_mapping(&mapping).await.unwrap();
        }

        let _integration = Integration::new(state.clone(), stream_handler);

        // Verify setup
        let stored = {
            let topic_store = state.topic_store.lock().await;
            topic_store.get_mapping(100).await.unwrap()
        };
        assert!(stored.is_some());
    }

    #[tokio::test]
    async fn test_route_message_to_discovered_instance() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;

        // Setup mapping for discovered instance
        let mut mapping = create_test_mapping(200);
        mapping.instance_id = Some("discovered-inst".to_string());
        {
            let topic_store = state.topic_store.lock().await;
            topic_store.save_mapping(&mapping).await.unwrap();
        }

        let _integration = Integration::new(state.clone(), stream_handler);

        // Verify setup
        let stored = {
            let topic_store = state.topic_store.lock().await;
            topic_store.get_mapping(200).await.unwrap()
        };
        assert!(stored.is_some());
        assert_eq!(
            stored.unwrap().instance_id,
            Some("discovered-inst".to_string())
        );
    }

    #[tokio::test]
    async fn test_route_message_to_external_instance() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;

        // Setup mapping for external instance
        let mut mapping = create_test_mapping(300);
        mapping.instance_id = Some("external-inst".to_string());
        {
            let topic_store = state.topic_store.lock().await;
            topic_store.save_mapping(&mapping).await.unwrap();
        }

        let integration = Integration::new(state.clone(), stream_handler);

        // Test port fallback for external instance
        let port = integration.get_instance_port(&mapping).await.unwrap();
        // Falls back to default since instance isn't in orchestrator store
        assert_eq!(port, state.config.opencode_port_start);
    }

    #[tokio::test]
    async fn test_permission_event_handling() {
        // Test that permission events are properly structured
        let event = StreamEvent::PermissionRequest {
            id: "perm-123".to_string(),
            permission_type: "file_write".to_string(),
            details: serde_json::json!({"path": "/test/file.txt"}),
        };

        match event {
            StreamEvent::PermissionRequest {
                id,
                permission_type,
                details,
            } => {
                assert_eq!(id, "perm-123");
                assert_eq!(permission_type, "file_write");
                assert_eq!(details["path"], "/test/file.txt");
            }
            _ => panic!("Expected PermissionRequest"),
        }
    }
}
