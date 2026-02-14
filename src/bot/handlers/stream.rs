use crate::bot::{BotState, Command};
use crate::types::error::{OutpostError, Result};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{MessageId, ThreadId};
use tracing::debug;

/// Extract topic_id from message, ensuring it's not the General topic
fn get_topic_id(msg: &Message) -> Result<i32> {
    let thread_id = msg.thread_id.ok_or_else(|| {
        OutpostError::telegram_error("This command must be used in a forum topic")
    })?;

    // General topic has ThreadId(MessageId(1))
    if thread_id.0 .0 == 1 {
        return Err(OutpostError::telegram_error(
            "Cannot use /stream in General topic",
        ));
    }

    Ok(thread_id.0 .0)
}

/// Format confirmation message based on streaming state
fn format_confirmation(enabled: bool) -> String {
    if enabled {
        "Streaming: ON\nYou will see real-time progress from OpenCode.".to_string()
    } else {
        "Streaming: OFF\nYou will only see final responses.".to_string()
    }
}

pub async fn handle_stream(
    bot: Bot,
    msg: Message,
    _cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    debug!(
        chat_id = msg.chat.id.0,
        topic_id = ?msg.thread_id.map(|t| t.0 .0),
        sender_id = ?msg.from.as_ref().map(|u| u.id.0),
        "Handling /stream"
    );
    let topic_id = get_topic_id(&msg)?;
    let chat_id = msg.chat.id;

    // Get current mapping to verify it exists
    let topic_store = state.topic_store.lock().await;
    let _mapping = topic_store
        .get_mapping(topic_id)
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?
        .ok_or_else(|| OutpostError::telegram_error("No active connection in this topic"))?;
    drop(topic_store);

    // Toggle streaming state
    let topic_store = state.topic_store.lock().await;
    let new_state = topic_store
        .toggle_streaming(topic_id)
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?;
    drop(topic_store);
    debug!(
        topic_id = topic_id,
        streaming_enabled = new_state,
        "Streaming toggled"
    );

    // Send confirmation message
    let confirmation = format_confirmation(new_state);
    bot.send_message(chat_id, confirmation)
        .message_thread_id(ThreadId(MessageId(topic_id)))
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::forum::TopicStore;
    use crate::orchestrator::container::{mock::MockRuntime, ContainerRuntime};
    use crate::orchestrator::store::OrchestratorStore;
    use crate::types::forum::TopicMapping;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;

    async fn create_test_state() -> (BotState, TempDir) {
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
            log_db_path: temp_dir.path().join("logs.db"),
            project_base_path: temp_dir.path().to_path_buf(),
            auto_create_project_dirs: true,
            api_port: 4200,
            api_key: None,
            docker_image: "ghcr.io/sst/opencode".to_string(),
            opencode_config_path: std::path::PathBuf::from("/tmp/oc-config"),
            container_port: 8080,
            env_passthrough: vec![],
        };

        let orchestrator_store = OrchestratorStore::new(&config.orchestrator_db_path)
            .await
            .unwrap();
        let topic_store = TopicStore::new(&config.topic_db_path).await.unwrap();

        let store_for_manager = orchestrator_store.clone();
        let port_pool = crate::orchestrator::port_pool::PortPool::new(4100, 10);
        let runtime: Arc<dyn ContainerRuntime> = Arc::new(MockRuntime::new());
        let instance_manager = crate::orchestrator::manager::InstanceManager::new(
            std::sync::Arc::new(config.clone()),
            store_for_manager,
            port_pool,
            runtime,
        )
        .await
        .unwrap();
        let bot_start_time = std::time::Instant::now();

        let state = BotState::new(
            orchestrator_store,
            topic_store,
            config,
            instance_manager,
            bot_start_time,
        );
        (state, temp_dir)
    }

    fn create_test_mapping(topic_id: i32, chat_id: i64, streaming_enabled: bool) -> TopicMapping {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        TopicMapping {
            topic_id,
            chat_id,
            project_path: "/test/project".to_string(),
            session_id: Some("ses_test".to_string()),
            instance_id: Some("inst_test".to_string()),
            streaming_enabled,
            topic_name_updated: false,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn test_cmd_stream_toggle_off_to_on() {
        let (state, _temp_dir) = create_test_state().await;

        let mapping = create_test_mapping(100, -1001234567890, false);
        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        let topic_store = state.topic_store.lock().await;
        let new_state = topic_store.toggle_streaming(100).await.unwrap();
        drop(topic_store);

        assert!(new_state);

        let topic_store = state.topic_store.lock().await;
        let retrieved = topic_store.get_mapping(100).await.unwrap().unwrap();
        assert!(retrieved.streaming_enabled);
    }

    #[tokio::test]
    async fn test_cmd_stream_toggle_on_to_off() {
        let (state, _temp_dir) = create_test_state().await;

        let mapping = create_test_mapping(101, -1001234567890, true);
        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        let topic_store = state.topic_store.lock().await;
        let new_state = topic_store.toggle_streaming(101).await.unwrap();
        drop(topic_store);

        assert!(!new_state);

        let topic_store = state.topic_store.lock().await;
        let retrieved = topic_store.get_mapping(101).await.unwrap().unwrap();
        assert!(!retrieved.streaming_enabled);
    }

    #[tokio::test]
    async fn test_cmd_stream_persistence_in_database() {
        let (state, _temp_dir) = create_test_state().await;

        let mapping = create_test_mapping(102, -1001234567890, false);
        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        // Toggle multiple times
        let topic_store = state.topic_store.lock().await;
        let state1 = topic_store.toggle_streaming(102).await.unwrap();
        drop(topic_store);
        assert!(state1);

        let topic_store = state.topic_store.lock().await;
        let state2 = topic_store.toggle_streaming(102).await.unwrap();
        drop(topic_store);
        assert!(!state2);

        // Verify final state persisted
        let topic_store = state.topic_store.lock().await;
        let retrieved = topic_store.get_mapping(102).await.unwrap().unwrap();
        assert!(!retrieved.streaming_enabled);
    }

    #[tokio::test]
    async fn test_cmd_stream_error_no_mapping() {
        let (state, _temp_dir) = create_test_state().await;

        let topic_store = state.topic_store.lock().await;
        let result = topic_store.toggle_streaming(999).await;
        drop(topic_store);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Mapping not found"));
    }

    #[tokio::test]
    async fn test_cmd_stream_confirmation_message_on() {
        let confirmation = format_confirmation(true);
        assert_eq!(
            confirmation,
            "Streaming: ON\nYou will see real-time progress from OpenCode."
        );
    }

    #[tokio::test]
    async fn test_cmd_stream_confirmation_message_off() {
        let confirmation = format_confirmation(false);
        assert_eq!(
            confirmation,
            "Streaming: OFF\nYou will only see final responses."
        );
    }

    #[tokio::test]
    async fn test_cmd_stream_multiple_toggles() {
        let (state, _temp_dir) = create_test_state().await;

        let mapping = create_test_mapping(103, -1001234567890, false);
        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        // Toggle 1: OFF -> ON
        let topic_store = state.topic_store.lock().await;
        let state1 = topic_store.toggle_streaming(103).await.unwrap();
        drop(topic_store);
        assert!(state1);

        // Toggle 2: ON -> OFF
        let topic_store = state.topic_store.lock().await;
        let state2 = topic_store.toggle_streaming(103).await.unwrap();
        drop(topic_store);
        assert!(!state2);

        // Toggle 3: OFF -> ON
        let topic_store = state.topic_store.lock().await;
        let state3 = topic_store.toggle_streaming(103).await.unwrap();
        drop(topic_store);
        assert!(state3);

        // Verify final state
        let topic_store = state.topic_store.lock().await;
        let retrieved = topic_store.get_mapping(103).await.unwrap().unwrap();
        assert!(retrieved.streaming_enabled);
    }
}
