use crate::bot::{BotState, Command};
use crate::types::error::{OutpostError, Result};
use crate::types::instance::InstanceType;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{MessageId, ThreadId};
use tracing::warn;

/// Extract topic_id from message, ensuring it's not the General topic
#[allow(dead_code)]
fn get_topic_id(msg: &Message) -> Result<i32> {
    let thread_id = msg.thread_id.ok_or_else(|| {
        OutpostError::telegram_error("This command must be used in a forum topic")
    })?;

    // General topic has ThreadId(MessageId(1))
    if thread_id.0 .0 == 1 {
        return Err(OutpostError::telegram_error(
            "Cannot disconnect from General topic",
        ));
    }

    Ok(thread_id.0 .0)
}

#[allow(dead_code)]
pub async fn handle_disconnect(
    bot: Bot,
    msg: Message,
    _cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    let topic_id = get_topic_id(&msg)?;
    let chat_id = msg.chat.id;

    let topic_store = state.topic_store.lock().await;
    let mapping = topic_store
        .get_mapping(topic_id)
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?
        .ok_or_else(|| OutpostError::telegram_error("No active connection in this topic"))?;
    drop(topic_store);

    if let Some(instance_id) = &mapping.instance_id {
        let store = state.orchestrator_store.lock().await;
        if let Some(instance_info) = store
            .get_instance(instance_id)
            .await
            .map_err(|e| OutpostError::database_error(e.to_string()))?
        {
            drop(store);
            if instance_info.instance_type == InstanceType::Managed {
                if let Err(e) = state.instance_manager.stop_instance(instance_id).await {
                    warn!("Failed to stop instance {}: {:?}", instance_id, e);
                }
            }
        }
    }

    let session_id = mapping.session_id.as_deref().unwrap_or("unknown");
    let confirmation = format!(
        "Disconnected from session {}. This topic will be deleted.",
        session_id
    );
    bot.send_message(chat_id, confirmation)
        .message_thread_id(ThreadId(MessageId(topic_id)))
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;

    let topic_store = state.topic_store.lock().await;
    topic_store
        .delete_mapping(topic_id)
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?;
    drop(topic_store);

    bot.delete_forum_topic(chat_id, ThreadId(MessageId(topic_id)))
        .await
        .map_err(|e| OutpostError::telegram_error(format!("Failed to delete topic: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::forum::TopicStore;
    use crate::orchestrator::store::OrchestratorStore;
    use crate::types::forum::TopicMapping;
    use crate::types::instance::{InstanceInfo, InstanceState};
    use std::path::PathBuf;
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
        let port_pool = crate::orchestrator::port_pool::PortPool::new(4100, 10);
        let instance_manager = crate::orchestrator::manager::InstanceManager::new(
            std::sync::Arc::new(config.clone()),
            store_for_manager,
            port_pool,
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

    #[tokio::test]
    async fn test_delete_mapping() {
        let (state, _temp_dir) = create_test_state().await;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mapping = TopicMapping {
            topic_id: 456,
            chat_id: -1001234567890,
            project_path: "/test/project".to_string(),
            session_id: Some("ses_test".to_string()),
            instance_id: Some("inst_test".to_string()),
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: now,
            updated_at: now,
        };

        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        let topic_store = state.topic_store.lock().await;
        topic_store.delete_mapping(456).await.unwrap();
        drop(topic_store);

        let topic_store = state.topic_store.lock().await;
        let result = topic_store.get_mapping(456).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_stop_managed_instance() {
        let (state, _temp_dir) = create_test_state().await;

        let instance = InstanceInfo {
            id: "inst_managed".to_string(),
            state: InstanceState::Running,
            instance_type: InstanceType::Managed,
            project_path: "/test/project".to_string(),
            port: 4100,
            pid: Some(12345),
            started_at: Some(1640000000),
            stopped_at: None,
        };

        let store = state.orchestrator_store.lock().await;
        store
            .save_instance(&instance, Some("ses_test"))
            .await
            .unwrap();
        drop(store);

        let store = state.orchestrator_store.lock().await;
        let result = store.get_instance("inst_managed").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().instance_type, InstanceType::Managed);
    }

    #[tokio::test]
    async fn test_dont_stop_discovered_instance() {
        let (state, _temp_dir) = create_test_state().await;

        let instance = InstanceInfo {
            id: "inst_discovered".to_string(),
            state: InstanceState::Running,
            instance_type: InstanceType::Discovered,
            project_path: "/test/project".to_string(),
            port: 4100,
            pid: Some(12345),
            started_at: Some(1640000000),
            stopped_at: None,
        };

        let store = state.orchestrator_store.lock().await;
        store
            .save_instance(&instance, Some("ses_test"))
            .await
            .unwrap();
        drop(store);

        let store = state.orchestrator_store.lock().await;
        let result = store.get_instance("inst_discovered").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().instance_type, InstanceType::Discovered);
    }

    #[tokio::test]
    async fn test_mapping_with_session_id() {
        let (state, _temp_dir) = create_test_state().await;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mapping = TopicMapping {
            topic_id: 789,
            chat_id: -1001234567890,
            project_path: "/test/project".to_string(),
            session_id: Some("ses_abc123".to_string()),
            instance_id: Some("inst_xyz".to_string()),
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: now,
            updated_at: now,
        };

        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        let topic_store = state.topic_store.lock().await;
        let result = topic_store.get_mapping(789).await.unwrap();
        assert!(result.is_some());
        let retrieved = result.unwrap();
        assert_eq!(retrieved.session_id, Some("ses_abc123".to_string()));
        assert_eq!(retrieved.instance_id, Some("inst_xyz".to_string()));
    }

    #[tokio::test]
    async fn test_mapping_without_session_id() {
        let (state, _temp_dir) = create_test_state().await;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mapping = TopicMapping {
            topic_id: 999,
            chat_id: -1001234567890,
            project_path: "/test/project".to_string(),
            session_id: None,
            instance_id: None,
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: now,
            updated_at: now,
        };

        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        let topic_store = state.topic_store.lock().await;
        let result = topic_store.get_mapping(999).await.unwrap();
        assert!(result.is_some());
        let retrieved = result.unwrap();
        assert_eq!(retrieved.session_id, None);
        assert_eq!(retrieved.instance_id, None);
    }

    #[tokio::test]
    async fn test_get_mapping_no_mapping_error() {
        let (state, _temp_dir) = create_test_state().await;

        let topic_store = state.topic_store.lock().await;
        let result = topic_store.get_mapping(12345).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_instance_type_comparison() {
        assert_eq!(InstanceType::Managed, InstanceType::Managed);
        assert_ne!(InstanceType::Managed, InstanceType::Discovered);
        assert_ne!(InstanceType::Managed, InstanceType::External);
    }
}
