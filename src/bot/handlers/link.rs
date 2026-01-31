use crate::bot::{BotState, Command};
use crate::types::error::{OutpostError, Result};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use teloxide::prelude::*;
use teloxide::types::{MessageId, ThreadId};
use tracing::debug;

fn get_topic_id(msg: &Message) -> Result<i32> {
    let thread_id = msg.thread_id.ok_or_else(|| {
        OutpostError::telegram_error("This command must be used in a forum topic")
    })?;

    if thread_id.0 .0 == 1 {
        return Err(OutpostError::telegram_error(
            "Cannot link from General topic",
        ));
    }

    Ok(thread_id.0 .0)
}

fn validate_path(path: &str) -> Result<PathBuf> {
    let expanded = shellexpand::tilde(path).into_owned();
    let path_buf = PathBuf::from(expanded);

    let absolute = path_buf.canonicalize().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            OutpostError::telegram_error(format!("Path not found: {}", path))
        } else if e.kind() == std::io::ErrorKind::PermissionDenied {
            OutpostError::telegram_error(format!("Permission denied: {}", path))
        } else {
            OutpostError::telegram_error(format!("Cannot access path: {}", path))
        }
    })?;

    let metadata = std::fs::metadata(&absolute).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            OutpostError::telegram_error(format!("Permission denied: {}", path))
        } else {
            OutpostError::telegram_error(format!("Cannot access path: {}", path))
        }
    })?;

    if !metadata.is_dir() {
        return Err(OutpostError::telegram_error(format!(
            "Path is not a directory: {}",
            path
        )));
    }

    Ok(absolute)
}

pub async fn handle_link(bot: Bot, msg: Message, cmd: Command, state: Arc<BotState>) -> Result<()> {
    debug!(
        chat_id = msg.chat.id.0,
        topic_id = ?msg.thread_id.map(|t| t.0 .0),
        sender_id = ?msg.from.as_ref().map(|u| u.id.0),
        "Handling /link"
    );
    let topic_id = get_topic_id(&msg)?;
    let chat_id = msg.chat.id;

    let Command::Link(path) = cmd else {
        return Err(OutpostError::telegram_error("Invalid command"));
    };

    let absolute_path = validate_path(&path)?;
    debug!(path = %absolute_path.display(), "Path validated for link");
    let path_str = absolute_path
        .to_str()
        .ok_or_else(|| OutpostError::telegram_error("Invalid path encoding"))?
        .to_string();

    let topic_store = state.topic_store.lock().await;
    let mut mapping = topic_store
        .get_mapping(topic_id)
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?
        .ok_or_else(|| OutpostError::telegram_error("No active connection in this topic"))?;
    drop(topic_store);

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    mapping.project_path = path_str.clone();
    mapping.updated_at = now;
    debug!(topic_id = topic_id, new_path = %path_str, "Updating mapping with new project path");

    let topic_store = state.topic_store.lock().await;
    topic_store
        .save_mapping(&mapping)
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?;
    drop(topic_store);
    debug!(topic_id = topic_id, "Link mapping saved");

    let confirmation = format!("Linked topic to {}", path_str);
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
    use crate::orchestrator::store::OrchestratorStore;
    use crate::types::forum::TopicMapping;
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

    #[test]
    fn test_validate_path_exists() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();
        let result = validate_path(path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_not_found() {
        let result = validate_path("/nonexistent/path/that/does/not/exist");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Path not found"));
    }

    #[test]
    fn test_validate_path_not_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "test").unwrap();

        let result = validate_path(file_path.to_str().unwrap());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not a directory"));
    }

    #[test]
    fn test_expand_tilde() {
        let home = dirs::home_dir().expect("Could not get home directory");
        let home_str = home.to_str().unwrap();

        let temp_dir = TempDir::new_in(&home).unwrap();
        let temp_path = temp_dir.path();
        let relative_path = format!("~/{}", temp_path.file_name().unwrap().to_str().unwrap());

        let result = validate_path(&relative_path);
        assert!(result.is_ok());
        let expanded = result.unwrap();
        assert!(expanded.to_str().unwrap().starts_with(home_str));
    }

    #[test]
    fn test_resolve_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let _current_dir = std::env::current_dir().unwrap();

        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = validate_path("./subdir");
        assert!(result.is_ok());

        std::env::set_current_dir(old_dir).unwrap();
    }

    #[tokio::test]
    async fn test_update_mapping() {
        let (state, _temp_dir) = create_test_state().await;

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mapping = TopicMapping {
            topic_id: 456,
            chat_id: -1001234567890,
            project_path: "/old/path".to_string(),
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

        let mut updated = mapping.clone();
        updated.project_path = "/new/path".to_string();
        updated.updated_at = now + 100;

        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&updated).await.unwrap();
        drop(topic_store);

        let topic_store = state.topic_store.lock().await;
        let result = topic_store.get_mapping(456).await.unwrap();
        assert!(result.is_some());
        let retrieved = result.unwrap();
        assert_eq!(retrieved.project_path, "/new/path");
        assert_eq!(retrieved.updated_at, now + 100);
    }

    #[test]
    fn test_validate_path_with_symlink() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        std::fs::create_dir(&target_dir).unwrap();

        let symlink_path = temp_dir.path().join("link");
        #[cfg(unix)]
        {
            use std::os::unix::fs as unix_fs;
            unix_fs::symlink(&target_dir, &symlink_path).unwrap();
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs as windows_fs;
            windows_fs::symlink_dir(&target_dir, &symlink_path).unwrap();
        }

        let result = validate_path(symlink_path.to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_absolute_path() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_str().unwrap();
        let result = validate_path(path);
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert!(resolved.is_absolute());
    }
}
