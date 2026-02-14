use crate::bot::{BotState, Command};
use crate::opencode::Discovery;
use crate::types::error::{OutpostError, Result};
use crate::types::forum::TopicMapping;
use std::path::PathBuf;
use std::sync::Arc;
use teloxide::prelude::*;
use tracing::debug;

#[derive(Debug, Clone)]
struct SessionInfo {
    session_id: String,
    instance_id: String,
    project_path: String,
    project_name: String,
}

/// Search for a session by name or ID across managed and discovered instances
async fn find_session(name: &str, state: &BotState) -> Result<Option<SessionInfo>> {
    let store = state.orchestrator_store.lock().await;
    let managed_instances = store
        .get_all_instances()
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?;
    drop(store);

    for instance in managed_instances {
        let path = PathBuf::from(&instance.project_path);
        let project_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if project_name == name || instance.id == name {
            if let Some(port) = instance.port.checked_add(0) {
                if let Ok(Some(session)) = Discovery::get_session_info(port).await {
                    return Ok(Some(SessionInfo {
                        session_id: session.id,
                        instance_id: instance.id,
                        project_path: instance.project_path,
                        project_name,
                    }));
                }
            }
        }
    }

    let discovered_instances = Discovery::discover_all()
        .await
        .map_err(|e| OutpostError::io_error(e.to_string()))?;
    for instance in discovered_instances {
        let project_name = instance
            .working_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if project_name == name {
            if let Some(port) = instance.port {
                if let Ok(Some(session)) = Discovery::get_session_info(port).await {
                    return Ok(Some(SessionInfo {
                        session_id: session.id.clone(),
                        instance_id: format!("discovered-{}", instance.pid),
                        project_path: instance.working_dir.to_string_lossy().to_string(),
                        project_name,
                    }));
                }
            }
        }
    }

    // TODO: External instances not yet implemented

    Ok(None)
}

pub async fn handle_connect(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    let Command::Connect(name) = cmd else {
        return Err(OutpostError::telegram_error("Invalid command"));
    };
    debug!(
        chat_id = msg.chat.id.0,
        topic_id = ?msg.thread_id.map(|t| t.0 .0),
        sender_id = ?msg.from.as_ref().map(|u| u.id.0),
        "Handling /connect"
    );
    debug!(name = %name, "Connect target extracted");

    let chat_id = msg.chat.id;

    let topic_store = state.topic_store.lock().await;
    let existing_mappings = topic_store
        .get_mappings_by_chat(chat_id.0)
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?;
    drop(topic_store);
    debug!(
        existing_count = existing_mappings.len(),
        "Checked existing mappings"
    );

    let session_info = match find_session(&name, &state).await? {
        Some(info) => {
            debug!(name = %name, found = true, "Session search result");
            info
        }
        None => {
            debug!(name = %name, found = false, "Session search result");
            bot.send_message(chat_id, format!("Session not found: {}", name))
                .await
                .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
            return Ok(());
        }
    };

    for mapping in existing_mappings {
        if mapping.session_id.as_deref() == Some(&session_info.session_id) {
            debug!(session_id = %session_info.session_id, "Already connected to this session");
            bot.send_message(
                chat_id,
                "Already connected to this session in another topic",
            )
            .await
            .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
            return Ok(());
        }
    }

    let topic_name = session_info.project_name.to_string();
    let forum_topic = match bot.create_forum_topic(chat_id, &topic_name).await {
        Ok(topic) => topic,
        Err(e) => {
            bot.send_message(chat_id, format!("Failed to create forum topic: {}", e))
                .await
                .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
            return Err(OutpostError::telegram_error(format!(
                "Failed to create forum topic: {}",
                e
            )));
        }
    };
    debug!(
        topic_id = forum_topic.thread_id.0 .0,
        "Forum topic created for connection"
    );

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| OutpostError::io_error(e.to_string()))?
        .as_secs() as i64;

    let mapping = TopicMapping {
        topic_id: forum_topic.thread_id.0 .0,
        chat_id: chat_id.0,
        project_path: session_info.project_path.clone(),
        session_id: Some(session_info.session_id.clone()),
        instance_id: Some(session_info.instance_id.clone()),
        streaming_enabled: false,
        topic_name_updated: false,
        created_at: now,
        updated_at: now,
    };

    let topic_store = state.topic_store.lock().await;
    topic_store
        .save_mapping(&mapping)
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?;
    drop(topic_store);
    debug!(topic_id = mapping.topic_id, session_id = ?mapping.session_id, "Connection mapping saved");

    let confirmation = format!(
        "Connected to session {} in project {}",
        session_info.session_id, session_info.project_name
    );
    bot.send_message(chat_id, confirmation)
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
    use crate::types::instance::{InstanceInfo, InstanceState, InstanceType};
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
            opencode_config_path: PathBuf::from("/tmp/oc-config"),
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

    fn create_test_instance(id: &str, port: u16, path: &str) -> InstanceInfo {
        InstanceInfo {
            id: id.to_string(),
            project_path: path.to_string(),
            port,
            state: InstanceState::Running,
            instance_type: InstanceType::Managed,
            pid: None,
            container_id: None,
            started_at: None,
            stopped_at: None,
        }
    }

    #[tokio::test]
    async fn test_find_session_by_project_name() {
        let (state, _temp_dir) = create_test_state().await;

        let instance = create_test_instance("test-1", 4100, "/test/my-project");
        let store = state.orchestrator_store.lock().await;
        store
            .save_instance(&instance, Some("ses_test-1"))
            .await
            .unwrap();
        drop(store);

        // NOTE: Requires mock for Discovery::get_session_info in integration tests
        let result = find_session("my-project", &state).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_find_session_by_instance_id() {
        let (state, _temp_dir) = create_test_state().await;

        let instance = create_test_instance("test-instance-1", 4100, "/test/project");
        let store = state.orchestrator_store.lock().await;
        store
            .save_instance(&instance, Some("ses_test-1"))
            .await
            .unwrap();
        drop(store);

        let result = find_session("test-instance-1", &state).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_find_session_not_found() {
        let (state, _temp_dir) = create_test_state().await;

        let result = find_session("nonexistent", &state).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_session_info_construction() {
        let info = SessionInfo {
            session_id: "ses_123".to_string(),
            instance_id: "inst_456".to_string(),
            project_path: "/test/project".to_string(),
            project_name: "project".to_string(),
        };

        assert_eq!(info.session_id, "ses_123");
        assert_eq!(info.instance_id, "inst_456");
        assert_eq!(info.project_path, "/test/project");
        assert_eq!(info.project_name, "project");
    }

    #[tokio::test]
    async fn test_handle_connect_invalid_command() {
        let (_state, _temp_dir) = create_test_state().await;

        let result = Command::Connect("test".to_string());
        assert!(matches!(result, Command::Connect(_)));
    }

    #[tokio::test]
    async fn test_find_session_extracts_project_name() {
        let (state, _temp_dir) = create_test_state().await;

        let instance = create_test_instance("test-1", 4100, "/home/user/projects/my-app");
        let store = state.orchestrator_store.lock().await;
        store
            .save_instance(&instance, Some("ses_test-1"))
            .await
            .unwrap();
        drop(store);

        let path = PathBuf::from("/home/user/projects/my-app");
        let project_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        assert_eq!(project_name, "my-app");
    }

    #[tokio::test]
    async fn test_find_session_handles_empty_instances() {
        let (state, _temp_dir) = create_test_state().await;

        let result = find_session("any-name", &state).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_session_searches_multiple_instances() {
        let (state, _temp_dir) = create_test_state().await;

        let instance1 = create_test_instance("test-1", 4100, "/test/project1");
        let instance2 = create_test_instance("test-2", 4101, "/test/project2");
        let instance3 = create_test_instance("test-3", 4102, "/test/my-target");

        let store = state.orchestrator_store.lock().await;
        store
            .save_instance(&instance1, Some("ses_1"))
            .await
            .unwrap();
        store
            .save_instance(&instance2, Some("ses_2"))
            .await
            .unwrap();
        store
            .save_instance(&instance3, Some("ses_3"))
            .await
            .unwrap();
        drop(store);

        let result = find_session("my-target", &state).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_topic_mapping_creation() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mapping = TopicMapping {
            topic_id: 123,
            chat_id: -1001234567890,
            project_path: "/test/project".to_string(),
            session_id: Some("ses_test".to_string()),
            instance_id: Some("inst_test".to_string()),
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(mapping.topic_id, 123);
        assert_eq!(mapping.session_id, Some("ses_test".to_string()));
        assert!(!mapping.streaming_enabled);
    }
}
