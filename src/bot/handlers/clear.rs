use crate::bot::{BotState, Command};
use crate::types::error::{OutpostError, Result};
use crate::types::instance::InstanceType;
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;
use tracing::debug;

pub async fn handle_clear(
    bot: Bot,
    msg: Message,
    _cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    debug!(
        chat_id = msg.chat.id.0,
        topic_id = ?msg.thread_id.map(|t| t.0 .0),
        sender_id = ?msg.from.as_ref().map(|u| u.id.0),
        "Handling /clear"
    );
    let chat_id = msg.chat.id;

    let topic_store = state.topic_store.lock().await;
    let stale_mappings = topic_store
        .get_stale_mappings(Duration::from_secs(7 * 24 * 60 * 60))
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?;
    drop(topic_store);
    debug!(stale_count = stale_mappings.len(), "Stale mappings found");

    let mut cleared_projects = Vec::new();

    for mapping in stale_mappings {
        if let Some(instance_id) = &mapping.instance_id {
            let store = state.orchestrator_store.lock().await;
            if let Some(instance_info) = store
                .get_instance(instance_id)
                .await
                .map_err(|e| OutpostError::database_error(e.to_string()))?
            {
                if instance_info.instance_type == InstanceType::Managed {
                    debug!(instance_id = %instance_id, "Stopping stale managed instance");
                    let _ = store
                        .update_state(instance_id, crate::types::instance::InstanceState::Stopped)
                        .await;
                }
            }
            drop(store);
        }

        let topic_store = state.topic_store.lock().await;
        topic_store
            .delete_mapping(mapping.topic_id)
            .await
            .map_err(|e| OutpostError::database_error(e.to_string()))?;
        drop(topic_store);
        debug!(topic_id = mapping.topic_id, "Stale mapping deleted");

        cleared_projects.push(mapping.project_path);
    }
    debug!(
        cleared_count = cleared_projects.len(),
        "Clear operation complete"
    );

    let response = if cleared_projects.is_empty() {
        "Cleanup Complete\n\nNo stale mappings found.".to_string()
    } else {
        let mut msg = format!(
            "Cleanup Complete\n\nCleared {} stale mappings:\n",
            cleared_projects.len()
        );
        for project in cleared_projects {
            msg.push_str(&format!("- {}\n", project));
        }
        msg.trim_end().to_string()
    };

    bot.send_message(chat_id, response)
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;

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
    async fn test_clear_with_no_stale_mappings() {
        let (state, _temp_dir) = create_test_state().await;

        let topic_store = state.topic_store.lock().await;
        let stale = topic_store
            .get_stale_mappings(Duration::from_secs(7 * 24 * 60 * 60))
            .await
            .unwrap();
        drop(topic_store);

        assert!(stale.is_empty());
    }

    #[tokio::test]
    async fn test_clear_with_stale_managed_instances() {
        let (state, _temp_dir) = create_test_state().await;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let old_time = now - (8 * 24 * 60 * 60);
        let mapping = TopicMapping {
            topic_id: 100,
            chat_id: -1001234567890,
            project_path: "/test/old-project".to_string(),
            session_id: Some("ses_old".to_string()),
            instance_id: Some("inst_managed".to_string()),
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: old_time,
            updated_at: old_time,
        };

        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        let instance = InstanceInfo {
            id: "inst_managed".to_string(),
            state: InstanceState::Running,
            instance_type: InstanceType::Managed,
            project_path: "/test/old-project".to_string(),
            port: 4100,
            pid: Some(12345),
            started_at: Some(old_time),
            stopped_at: None,
        };

        let store = state.orchestrator_store.lock().await;
        store
            .save_instance(&instance, Some("ses_old"))
            .await
            .unwrap();
        drop(store);

        let topic_store = state.topic_store.lock().await;
        let stale = topic_store
            .get_stale_mappings(Duration::from_secs(7 * 24 * 60 * 60))
            .await
            .unwrap();
        drop(topic_store);

        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].project_path, "/test/old-project");
        assert_eq!(stale[0].instance_id, Some("inst_managed".to_string()));
    }

    #[tokio::test]
    async fn test_clear_with_stale_discovered_instances() {
        let (state, _temp_dir) = create_test_state().await;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let old_time = now - (8 * 24 * 60 * 60);
        let mapping = TopicMapping {
            topic_id: 200,
            chat_id: -1001234567890,
            project_path: "/test/discovered-project".to_string(),
            session_id: Some("ses_discovered".to_string()),
            instance_id: Some("inst_discovered".to_string()),
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: old_time,
            updated_at: old_time,
        };

        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        let instance = InstanceInfo {
            id: "inst_discovered".to_string(),
            state: InstanceState::Running,
            instance_type: InstanceType::Discovered,
            project_path: "/test/discovered-project".to_string(),
            port: 4101,
            pid: Some(54321),
            started_at: Some(old_time),
            stopped_at: None,
        };

        let store = state.orchestrator_store.lock().await;
        store
            .save_instance(&instance, Some("ses_discovered"))
            .await
            .unwrap();
        drop(store);

        let topic_store = state.topic_store.lock().await;
        let stale = topic_store
            .get_stale_mappings(Duration::from_secs(7 * 24 * 60 * 60))
            .await
            .unwrap();
        drop(topic_store);

        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].instance_id, Some("inst_discovered".to_string()));
    }

    #[tokio::test]
    async fn test_clear_formatting_empty() {
        let (state, _temp_dir) = create_test_state().await;

        let topic_store = state.topic_store.lock().await;
        let stale = topic_store
            .get_stale_mappings(Duration::from_secs(7 * 24 * 60 * 60))
            .await
            .unwrap();
        drop(topic_store);

        let response = if stale.is_empty() {
            "Cleanup Complete\n\nNo stale mappings found.".to_string()
        } else {
            let mut msg = format!(
                "Cleanup Complete\n\nCleared {} stale mappings:\n",
                stale.len()
            );
            for mapping in stale {
                msg.push_str(&format!("- {}\n", mapping.project_path));
            }
            msg.trim_end().to_string()
        };

        assert_eq!(response, "Cleanup Complete\n\nNo stale mappings found.");
    }

    #[tokio::test]
    async fn test_clear_formatting_with_mappings() {
        let (state, _temp_dir) = create_test_state().await;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let old_time = now - (8 * 24 * 60 * 60);

        for i in 0..3 {
            let mapping = TopicMapping {
                topic_id: 300 + i,
                chat_id: -1001234567890,
                project_path: format!("/test/project-{}", i),
                session_id: Some(format!("ses_{}", i)),
                instance_id: None,
                streaming_enabled: false,
                topic_name_updated: false,
                created_at: old_time,
                updated_at: old_time,
            };

            let topic_store = state.topic_store.lock().await;
            topic_store.save_mapping(&mapping).await.unwrap();
            drop(topic_store);
        }

        let topic_store = state.topic_store.lock().await;
        let stale = topic_store
            .get_stale_mappings(Duration::from_secs(7 * 24 * 60 * 60))
            .await
            .unwrap();
        drop(topic_store);

        let mut response = format!(
            "Cleanup Complete\n\nCleared {} stale mappings:\n",
            stale.len()
        );
        for mapping in stale {
            response.push_str(&format!("- {}\n", mapping.project_path));
        }
        let response = response.trim_end().to_string();

        assert!(response.contains("Cleanup Complete"));
        assert!(response.contains("Cleared 3 stale mappings:"));
        assert!(response.contains("- /test/project-0"));
        assert!(response.contains("- /test/project-1"));
        assert!(response.contains("- /test/project-2"));
    }

    #[tokio::test]
    async fn test_clear_error_handling_missing_instance() {
        let (state, _temp_dir) = create_test_state().await;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let old_time = now - (8 * 24 * 60 * 60);

        let mapping = TopicMapping {
            topic_id: 400,
            chat_id: -1001234567890,
            project_path: "/test/missing-instance".to_string(),
            session_id: Some("ses_missing".to_string()),
            instance_id: Some("inst_nonexistent".to_string()),
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: old_time,
            updated_at: old_time,
        };

        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        let topic_store = state.topic_store.lock().await;
        let stale = topic_store
            .get_stale_mappings(Duration::from_secs(7 * 24 * 60 * 60))
            .await
            .unwrap();
        drop(topic_store);

        assert_eq!(stale.len(), 1);

        let store = state.orchestrator_store.lock().await;
        let result = store.get_instance("inst_nonexistent").await.unwrap();
        drop(store);

        assert!(result.is_none());
    }
}
