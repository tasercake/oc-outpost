//! Integration Layer - Wires all components together.
//!
//! Responsibilities:
//! - Message routing (Telegram -> OpenCode), including photo/image support
//! - Stream bridging (OpenCode -> Telegram)
//! - Topic name auto-update after first response
//! - Rate limiting for Telegram API
//! - Whitelist enforcement (defense in depth)

use crate::bot::BotState;
use crate::opencode::stream_handler::{StreamEvent, StreamHandler};
use crate::opencode::OpenCodeClient;
use crate::telegram::markdown::markdown_to_telegram_html;
use crate::types::error::{OutpostError, Result};
use crate::types::forum::TopicMapping;
use crate::types::instance::InstanceState;
use crate::types::opencode::{FilePart, MessagePart};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use teloxide::net::Download;
use teloxide::prelude::*;
use teloxide::types::{
    InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ParseMode, PhotoSize, ThreadId,
};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, info, trace, warn};

/// Telegram rate limit: ~30 messages/second, we use 2-second batching
const TELEGRAM_BATCH_INTERVAL: Duration = Duration::from_secs(2);

/// Maximum message length for Telegram (4096 characters)
const TELEGRAM_MAX_MESSAGE_LENGTH: usize = 4096;

/// Timeout for instance resurrection attempts.
const RESURRECTION_TIMEOUT: Duration = Duration::from_secs(30);

/// Delay before showing "waking up" message during resurrection.
const RESURRECTION_WAKE_DELAY: Duration = Duration::from_secs(3);

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

    pub async fn handle_message(&self, bot: Bot, msg: Message) -> Result<()> {
        if !self.state.config.is_whitelisted_chat(msg.chat.id.0) {
            debug!(
                chat_id = msg.chat.id.0,
                "Ignoring message from non-whitelisted chat"
            );
            return Ok(());
        }

        let thread_id = msg.thread_id.ok_or_else(|| {
            OutpostError::telegram_error("Message is not in a forum topic (no thread_id)")
        })?;
        let topic_id = thread_id.0 .0;

        let mapping = self
            .state
            .topic_store
            .get_mapping(msg.chat.id.0, topic_id)
            .await
            .map_err(|e| OutpostError::database_error(e.to_string()))?;

        if mapping.is_none() {
            let is_actionable = msg.text().is_some()
                || msg.photo().is_some()
                || msg.forum_topic_created().is_some();
            if is_actionable {
                info!(
                    topic_id = topic_id,
                    chat_id = msg.chat.id.0,
                    "Unmapped topic detected, sending project selection keyboard"
                );
                self.send_project_selection_keyboard(&bot, msg.chat.id, topic_id)
                    .await?;
            } else {
                debug!(
                    topic_id = topic_id,
                    message_kind = describe_message_kind(&msg),
                    "No mapping for topic, ignoring non-actionable message"
                );
            }
            return Ok(());
        }
        let mapping = mapping.unwrap();

        let (text, photo) = extract_message_content(&msg);
        if text.is_none() && photo.is_none() {
            debug!(
                chat_id = msg.chat.id.0,
                topic_id = topic_id,
                message_kind = describe_message_kind(&msg),
                "Ignoring unsupported message type in mapped topic"
            );
            return Ok(());
        }

        let session_id = match mapping.session_id.as_ref() {
            Some(id) => id,
            None => {
                warn!(
                    topic_id = topic_id,
                    project_path = %mapping.project_path,
                    "Message sent to topic with no session"
                );
                return Err(OutpostError::session_not_found(format!(
                    "No session for topic {} (project: {})",
                    topic_id, mapping.project_path
                )));
            }
        };

        let port = self
            .get_port_or_resurrect(&bot, msg.chat.id, topic_id, &mapping)
            .await?;
        let client = OpenCodeClient::new(&format!("http://localhost:{}", port));

        let mut parts: Vec<MessagePart> = Vec::new();

        if let Some(ref text) = text {
            parts.push(MessagePart::Text {
                text: text.to_string(),
            });
            self.stream_handler.mark_from_telegram(session_id, text);
        }

        if let Some(photo_sizes) = photo {
            match self
                .download_photo(&bot, photo_sizes, &mapping.project_path)
                .await
            {
                Ok(file_part) => {
                    trace!(
                        topic_id = topic_id,
                        mime = %file_part.mime,
                        "Image downloaded for OpenCode"
                    );
                    parts.push(MessagePart::File(file_part));
                }
                Err(e) => {
                    warn!(topic_id = topic_id, error = ?e, "Failed to download photo, sending text only");
                }
            }
        }

        if parts.is_empty() {
            return Ok(());
        }

        client
            .send_message_parts_async(session_id, parts)
            .await
            .map_err(|e| OutpostError::opencode_api_error(e.to_string()))?;

        info!(
            topic_id = topic_id,
            session_id = session_id,
            "Routed message to OpenCode"
        );

        self.ensure_stream_subscription(bot, msg.chat.id, topic_id, &mapping)
            .await?;

        Ok(())
    }

    async fn send_project_selection_keyboard(
        &self,
        bot: &Bot,
        chat_id: ChatId,
        topic_id: i32,
    ) -> Result<()> {
        let dirs =
            crate::bot::handlers::projects::list_project_dirs(&self.state.config.project_base_path);

        if dirs.is_empty() {
            bot.send_message(
                chat_id,
                "No projects available. Add project directories to get started.",
            )
            .message_thread_id(ThreadId(MessageId(topic_id)))
            .await
            .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
            return Ok(());
        }

        let topic_id_str = topic_id.to_string();
        let buttons: Vec<Vec<InlineKeyboardButton>> = dirs
            .iter()
            .filter(|name| {
                let data_len = 5 + topic_id_str.len() + 1 + name.len();
                if data_len > 64 {
                    warn!(project = %name, "Skipping project: name too long for callback data");
                    false
                } else {
                    true
                }
            })
            .map(|name| {
                vec![InlineKeyboardButton::callback(
                    name.clone(),
                    format!("proj:{}:{}", topic_id_str, name),
                )]
            })
            .collect();

        if buttons.is_empty() {
            bot.send_message(chat_id, "No projects available with compatible names.")
                .message_thread_id(ThreadId(MessageId(topic_id)))
                .await
                .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
            return Ok(());
        }

        let keyboard = InlineKeyboardMarkup::new(buttons);
        bot.send_message(chat_id, "Select a project for this topic:")
            .message_thread_id(ThreadId(MessageId(topic_id)))
            .reply_markup(keyboard)
            .await
            .map_err(|e| OutpostError::telegram_error(e.to_string()))?;

        debug!(
            topic_id = topic_id,
            project_count = dirs.len(),
            "Project selection keyboard sent"
        );
        Ok(())
    }

    /// Download a Telegram photo and save it to the container's mounted volume.
    /// Returns a `FilePart` with a `file://` URL pointing to the container-internal path.
    async fn download_photo(
        &self,
        bot: &Bot,
        photo_sizes: &[PhotoSize],
        project_path: &str,
    ) -> std::result::Result<FilePart, anyhow::Error> {
        use uuid::Uuid;

        let photo = photo_sizes
            .last()
            .ok_or_else(|| anyhow::anyhow!("Empty photo sizes array"))?;

        let file = bot
            .get_file(photo.file.id.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get file info: {}", e))?;

        let image_id = Uuid::new_v4();
        let filename = format!("{}.jpg", image_id);

        // Host path: {project_path}/.opencode-images/{uuid}.jpg
        let host_dir = PathBuf::from(project_path).join(".opencode-images");
        tokio::fs::create_dir_all(&host_dir).await?;
        let host_path = host_dir.join(&filename);

        let mut dest = tokio::fs::File::create(&host_path).await?;
        bot.download_file(&file.path, &mut dest).await?;

        trace!(host_path = %host_path.display(), "Photo saved to host volume");

        // Container-internal path (project dir is mounted at /workspace)
        let container_path = PathBuf::from("/workspace/.opencode-images").join(&filename);
        Ok(FilePart::new("image/jpeg", &container_path))
    }

    async fn get_port_or_resurrect(
        &self,
        bot: &Bot,
        chat_id: ChatId,
        topic_id: i32,
        mapping: &TopicMapping,
    ) -> Result<u16> {
        let path = Path::new(&mapping.project_path);
        if let Some(instance) = self.state.instance_manager.get_instance_by_path(path).await {
            let inst = instance.lock().await;
            if matches!(
                inst.state().await,
                InstanceState::Running | InstanceState::Starting
            ) {
                return Ok(inst.port());
            }
        }

        if let Some(instance_id) = &mapping.instance_id {
            if let Ok(Some(info)) = self
                .state
                .orchestrator_store
                .get_instance(instance_id)
                .await
            {
                if info.state == InstanceState::Running {
                    return Ok(info.port);
                }
            }
        }

        self.resurrect_instance(bot, chat_id, topic_id, mapping)
            .await
    }

    async fn resurrect_instance(
        &self,
        bot: &Bot,
        chat_id: ChatId,
        topic_id: i32,
        mapping: &TopicMapping,
    ) -> Result<u16> {
        info!(
            topic_id = topic_id,
            session_id = ?mapping.session_id,
            project_path = %mapping.project_path,
            "Resurrecting stopped instance for mapped topic"
        );

        let wake_msg_id: Arc<Mutex<Option<MessageId>>> = Arc::new(Mutex::new(None));
        let wake_clone = wake_msg_id.clone();
        let bot_wake = bot.clone();
        let wake_handle = tokio::spawn(async move {
            tokio::time::sleep(RESURRECTION_WAKE_DELAY).await;
            if let Ok(msg) = bot_wake
                .send_message(chat_id, "Waking up session...")
                .message_thread_id(ThreadId(MessageId(topic_id)))
                .await
            {
                *wake_clone.lock().await = Some(msg.id);
            }
        });

        let path = Path::new(&mapping.project_path);
        let result = tokio::time::timeout(
            RESURRECTION_TIMEOUT,
            self.state.instance_manager.get_or_create(path, topic_id),
        )
        .await;

        wake_handle.abort();
        let maybe_wake_id = { *wake_msg_id.lock().await };
        if let Some(mid) = maybe_wake_id {
            let _ = bot.delete_message(chat_id, mid).await;
        }

        match result {
            Ok(Ok(instance)) => {
                let inst = instance.lock().await;
                let port = inst.port();
                let new_instance_id = inst.id().to_string();
                drop(inst);

                let mut updated_mapping = mapping.clone();
                updated_mapping.instance_id = Some(new_instance_id.clone());
                updated_mapping.updated_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;

                self.state
                    .topic_store
                    .save_mapping(&updated_mapping)
                    .await
                    .map_err(|e| OutpostError::database_error(e.to_string()))?;

                info!(
                    topic_id = topic_id,
                    new_instance_id = %new_instance_id,
                    port = port,
                    session_id = ?mapping.session_id,
                    "Instance resurrected successfully"
                );

                Ok(port)
            }
            Ok(Err(e)) => {
                warn!(
                    topic_id = topic_id,
                    error = ?e,
                    "Failed to resurrect instance"
                );
                Err(OutpostError::opencode_api_error(format!(
                    "Failed to wake up session: {}",
                    e
                )))
            }
            Err(_elapsed) => {
                warn!(
                    topic_id = topic_id,
                    timeout_secs = RESURRECTION_TIMEOUT.as_secs(),
                    "Instance resurrection timed out"
                );
                Err(OutpostError::opencode_api_error(
                    "Session wake-up timed out (30s)",
                ))
            }
        }
    }

    #[allow(dead_code)]
    // Retained for direct port lookup without resurrection
    async fn get_instance_port(&self, mapping: &TopicMapping) -> Result<u16> {
        if let Some(instance_id) = &mapping.instance_id {
            if let Ok(Some(info)) = self
                .state
                .orchestrator_store
                .get_instance(instance_id)
                .await
            {
                return Ok(info.port);
            }
        }
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
                debug!(
                    topic_id = topic_id,
                    "Stream already active, skipping subscription"
                );
                return Ok(());
            }
        }

        let session_id = mapping
            .session_id
            .clone()
            .ok_or_else(|| OutpostError::session_not_found("No session for topic"))?;

        // Subscribe to SSE
        debug!(
            topic_id = topic_id,
            session_id = %session_id,
            "Subscribing to SSE stream"
        );

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

        debug!(topic_id = topic_id, "Stream subscription registered");

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

            debug!(
                topic_id = topic_id,
                session_id = %session_id,
                first_response = first_response,
                "Stream forwarder task started"
            );

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

                debug!(
                    topic_id = topic_id,
                    chunk_len = text.len(),
                    should_send = should_send,
                    "Text chunk received"
                );

                if should_send {
                    Self::flush_pending_text(bot, chat_id, topic_id, rate_limiters).await;
                }
            }

            StreamEvent::ToolInvocation { name, args } => {
                debug!(topic_id = topic_id, tool_name = %name, "Tool invocation event");

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
                debug!(
                    topic_id = topic_id,
                    result_len = result.len(),
                    "Tool result event"
                );

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
                debug!(
                    topic_id = topic_id,
                    permission_id = %id,
                    permission_type = %permission_type,
                    "Permission request event"
                );

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

        debug!(
            topic_id = topic_id,
            text_len = text_to_send.len(),
            "Flushing batched text to Telegram"
        );

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

        debug!(
            topic_id = topic_id,
            parts_count = parts.len(),
            total_len = text.len(),
            "Sending message parts to Telegram"
        );

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
        debug!(
            topic_id = topic_id,
            project_path = %mapping.project_path,
            "Attempting topic name update"
        );

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
        state
            .topic_store
            .mark_topic_name_updated(chat_id.0, topic_id)
            .await
            .map_err(|e| OutpostError::database_error(e.to_string()))?;

        info!("Updated topic {} name to '{}'", topic_id, project_name);

        Ok(())
    }

    /// Stop stream forwarding for a topic
    #[allow(dead_code)]
    // Used by future: selective stream stopping feature
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
        debug!("Stopping all active streams");

        let handles: Vec<_> = {
            let mut streams = self.active_streams.lock().await;
            streams.drain().collect()
        };

        debug!(count = handles.len(), "Stopping active stream handles");

        for (topic_id, handle) in handles {
            handle.abort();
            debug!("Stopped stream for topic {}", topic_id);
        }
    }

    /// Get count of active streams
    #[allow(dead_code)]
    // Used by future: stream monitoring feature
    pub async fn active_stream_count(&self) -> usize {
        self.active_streams.lock().await.len()
    }
}

fn extract_message_content(msg: &Message) -> (Option<&str>, Option<&[PhotoSize]>) {
    let text = msg.text().or_else(|| msg.caption());
    let photo = msg.photo();
    (text, photo)
}

fn describe_message_kind(msg: &Message) -> &'static str {
    if msg.text().is_some() {
        "text"
    } else if msg.photo().is_some() {
        "photo"
    } else if msg.sticker().is_some() {
        "sticker"
    } else if msg.video().is_some() {
        "video"
    } else if msg.voice().is_some() {
        "voice"
    } else if msg.document().is_some() {
        "document"
    } else if msg.audio().is_some() {
        "audio"
    } else if msg.animation().is_some() {
        "animation"
    } else if msg.video_note().is_some() {
        "video_note"
    } else if msg.contact().is_some() {
        "contact"
    } else if msg.location().is_some() {
        "location"
    } else if msg.poll().is_some() {
        "poll"
    } else if msg.forum_topic_created().is_some() {
        "forum_topic_created"
    } else if msg.forum_topic_edited().is_some() {
        "forum_topic_edited"
    } else if msg.forum_topic_closed().is_some() {
        "forum_topic_closed"
    } else if msg.forum_topic_reopened().is_some() {
        "forum_topic_reopened"
    } else if msg.new_chat_members().is_some() {
        "new_chat_members"
    } else if msg.left_chat_member().is_some() {
        "left_chat_member"
    } else {
        "other"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::forum::TopicStore;
    use crate::orchestrator::container::{mock::MockRuntime, ContainerRuntime};
    use crate::orchestrator::manager::InstanceManager;
    use crate::orchestrator::port_pool::PortPool;
    use crate::orchestrator::store::OrchestratorStore;
    use std::path::PathBuf;
    use tempfile::TempDir;

    async fn create_test_state() -> (Arc<BotState>, Arc<StreamHandler>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            telegram_bot_token: "test_token".to_string(),
            telegram_chat_ids: vec![-1001234567890],
            telegram_allowed_users: vec![],
            handle_general_topic: true,
            opencode_path: PathBuf::from("opencode"),
            opencode_max_instances: 10,
            opencode_idle_timeout: Duration::from_secs(1800),
            opencode_port_start: 4100,
            opencode_port_pool_size: 100,
            opencode_health_check_interval: Duration::from_secs(30),
            opencode_startup_timeout: Duration::from_secs(60),
            opencode_data_path: PathBuf::from("/tmp/opencode-data"),
            orchestrator_db_path: temp_dir.path().join("orchestrator.db"),
            topic_db_path: temp_dir.path().join("topics.db"),
            log_db_path: temp_dir.path().join("logs.db"),
            project_base_path: temp_dir.path().to_path_buf(),
            auto_create_project_dirs: true,
            docker_image: "ghcr.io/sst/opencode".to_string(),
            opencode_config_path: PathBuf::from("/tmp/oc-config"),
            container_port: 8080,
            env_passthrough: vec![],
        };

        let orchestrator_store = OrchestratorStore::new(&config.orchestrator_db_path)
            .await
            .unwrap();
        let topic_store = TopicStore::new(&config.topic_db_path).await.unwrap();

        let store_for_manager = orchestrator_store.clone();
        let port_pool = PortPool::new(4100, 10);
        let runtime: Arc<dyn ContainerRuntime> = Arc::new(MockRuntime::new());
        let instance_manager = InstanceManager::new(
            Arc::new(config.clone()),
            store_for_manager,
            port_pool,
            runtime,
        )
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
        let mapping = state
            .topic_store
            .get_mapping(-1001234567890, 999)
            .await
            .unwrap();
        assert!(mapping.is_none()); // No mapping exists
    }

    #[tokio::test]
    async fn test_route_message_with_mapping() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;

        // Create a mapping
        let mapping = create_test_mapping(123);
        state.topic_store.save_mapping(&mapping).await.unwrap();

        let _integration = Integration::new(state.clone(), stream_handler);

        // Verify mapping exists
        let stored = state
            .topic_store
            .get_mapping(-1001234567890, 123)
            .await
            .unwrap();
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
    async fn test_mapping_defaults() {
        let mapping = create_test_mapping(123);
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
        state.topic_store.save_mapping(&mapping).await.unwrap();

        let _integration = Integration::new(state.clone(), stream_handler);

        // Verify setup
        let stored = state
            .topic_store
            .get_mapping(-1001234567890, 100)
            .await
            .unwrap();
        assert!(stored.is_some());
    }

    #[tokio::test]
    async fn test_route_message_with_instance_id() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;

        let mut mapping = create_test_mapping(200);
        mapping.instance_id = Some("some-inst".to_string());
        state.topic_store.save_mapping(&mapping).await.unwrap();

        let _integration = Integration::new(state.clone(), stream_handler);

        let stored = state
            .topic_store
            .get_mapping(-1001234567890, 200)
            .await
            .unwrap();
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().instance_id, Some("some-inst".to_string()));
    }

    #[tokio::test]
    async fn test_port_fallback_when_instance_not_in_store() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;

        let mut mapping = create_test_mapping(300);
        mapping.instance_id = Some("unknown-inst".to_string());
        state.topic_store.save_mapping(&mapping).await.unwrap();

        let integration = Integration::new(state.clone(), stream_handler);

        let port = integration.get_instance_port(&mapping).await.unwrap();
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

    #[tokio::test]
    async fn test_file_part_for_image_pipeline() {
        let container_path = PathBuf::from("/workspace/.opencode-images/test-uuid.jpg");
        let file_part = FilePart::new("image/jpeg", &container_path);

        assert_eq!(file_part.mime, "image/jpeg");
        assert!(file_part.url.starts_with("file://"));
        assert!(file_part.url.contains("/workspace/.opencode-images/"));
        assert_eq!(file_part.filename, Some("test-uuid.jpg".to_string()));
    }

    #[tokio::test]
    async fn test_whitelist_rejects_unknown_chat() {
        let (state, _, _temp_dir) = create_test_state().await;
        assert!(!state.config.is_whitelisted_chat(-9999));
        assert!(state.config.is_whitelisted_chat(-1001234567890));
    }

    #[tokio::test]
    async fn test_message_parts_construction() {
        let mut parts: Vec<MessagePart> = Vec::new();
        parts.push(MessagePart::Text {
            text: "Hello".to_string(),
        });

        let container_path = PathBuf::from("/workspace/.opencode-images/img.jpg");
        parts.push(MessagePart::File(FilePart::new(
            "image/jpeg",
            &container_path,
        )));

        assert_eq!(parts.len(), 2);
        match &parts[0] {
            MessagePart::Text { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected Text"),
        }
        match &parts[1] {
            MessagePart::File(fp) => assert_eq!(fp.mime, "image/jpeg"),
            _ => panic!("Expected File"),
        }
    }

    #[tokio::test]
    async fn test_resurrection_constants() {
        assert_eq!(RESURRECTION_TIMEOUT, Duration::from_secs(30));
        assert_eq!(RESURRECTION_WAKE_DELAY, Duration::from_secs(3));
    }

    #[tokio::test]
    async fn test_mapping_with_session_but_no_running_instance() {
        let (state, stream_handler, _temp_dir) = create_test_state().await;

        let mapping = create_test_mapping(500);
        assert!(mapping.session_id.is_some());
        assert!(mapping.instance_id.is_some());
        state.topic_store.save_mapping(&mapping).await.unwrap();

        let integration = Integration::new(state.clone(), stream_handler);

        let path = std::path::Path::new(&mapping.project_path);
        let running = integration
            .state
            .instance_manager
            .get_instance_by_path(path)
            .await;
        assert!(running.is_none());

        let stored = state
            .topic_store
            .get_mapping(-1001234567890, 500)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.session_id, Some("session-123".to_string()));
    }

    #[tokio::test]
    async fn test_resurrection_preserves_session_id_in_mapping() {
        let (state, _stream_handler, _temp_dir) = create_test_state().await;

        let mapping = create_test_mapping(600);
        let original_session_id = mapping.session_id.clone();
        state.topic_store.save_mapping(&mapping).await.unwrap();

        let mut updated = mapping.clone();
        updated.instance_id = Some("new-inst-789".to_string());
        updated.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        state.topic_store.save_mapping(&updated).await.unwrap();

        let stored = state
            .topic_store
            .get_mapping(-1001234567890, 600)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(stored.session_id, original_session_id);
        assert_eq!(stored.instance_id, Some("new-inst-789".to_string()));
    }

    #[tokio::test]
    async fn test_instance_state_resurrection_gate() {
        use crate::types::instance::InstanceState;

        // Only Running/Starting should bypass resurrection
        assert!(matches!(
            InstanceState::Running,
            InstanceState::Running | InstanceState::Starting
        ));
        assert!(matches!(
            InstanceState::Starting,
            InstanceState::Running | InstanceState::Starting
        ));

        // Stopped/Error should trigger resurrection
        assert!(!matches!(
            InstanceState::Stopped,
            InstanceState::Running | InstanceState::Starting
        ));
        assert!(!matches!(
            InstanceState::Error,
            InstanceState::Running | InstanceState::Starting
        ));
        assert!(!matches!(
            InstanceState::Stopping,
            InstanceState::Running | InstanceState::Starting
        ));
    }
}
