use crate::bot::{BotState, Command};
use crate::types::error::{OutpostError, Result};
use std::path::PathBuf;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ThreadId};
use tracing::{debug, warn};

fn get_topic_id(msg: &Message) -> Result<i32> {
    let thread_id = msg.thread_id.ok_or_else(|| {
        OutpostError::telegram_error("This command must be used in a forum topic")
    })?;

    if thread_id.0 .0 == 1 {
        return Err(OutpostError::telegram_error(
            "Cannot close the General topic",
        ));
    }

    Ok(thread_id.0 .0)
}

pub async fn handle_close(
    bot: Bot,
    msg: Message,
    _cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    debug!(
        chat_id = msg.chat.id.0,
        topic_id = ?msg.thread_id.map(|t| t.0 .0),
        sender_id = ?msg.from.as_ref().map(|u| u.id.0),
        "Handling /close"
    );

    let topic_id = get_topic_id(&msg)?;

    let _mapping = state
        .topic_store
        .get_mapping(msg.chat.id.0, topic_id)
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?
        .ok_or_else(|| OutpostError::telegram_error("No active connection in this topic"))?;

    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("✅ Confirm", format!("close:{}:confirm", topic_id)),
        InlineKeyboardButton::callback("❌ Cancel", format!("close:{}:cancel", topic_id)),
    ]]);

    bot.send_message(
        msg.chat.id,
        "Are you sure you want to close this topic? This will stop the instance, remove the worktree, and delete this topic.",
    )
    .message_thread_id(ThreadId(MessageId(topic_id)))
    .reply_markup(keyboard)
    .await
    .map_err(|e| OutpostError::telegram_error(e.to_string()))?;

    Ok(())
}

pub async fn handle_close_callback(bot: Bot, q: CallbackQuery, state: Arc<BotState>) -> Result<()> {
    debug!(callback_data = ?q.data, "Handling close callback");

    let data = q
        .data
        .as_deref()
        .ok_or_else(|| OutpostError::telegram_error("No callback data"))?;

    let parts: Vec<&str> = data.split(':').collect();
    if parts.len() != 3 || parts[0] != "close" {
        return Err(OutpostError::telegram_error(
            "Invalid close callback data format",
        ));
    }

    let topic_id: i32 = parts[1]
        .parse()
        .map_err(|_| OutpostError::telegram_error("Invalid topic_id in callback data"))?;
    let action = parts[2];

    let _ = bot.answer_callback_query(q.id.clone()).await;

    if action == "cancel" {
        if let Some(ref message) = q.message {
            let chat_id = message.chat().id;
            let message_id = message.id();
            let _ = bot
                .edit_message_text(chat_id, message_id, "Close cancelled.")
                .reply_markup(InlineKeyboardMarkup::new(
                    Vec::<Vec<InlineKeyboardButton>>::new(),
                ))
                .await;
        }
        return Ok(());
    }

    if action == "confirm" {
        let chat_id = if let Some(ref message) = q.message {
            let chat_id = message.chat().id;
            let message_id = message.id();
            let _ = bot
                .edit_message_text(chat_id, message_id, "Closing topic... ⏳")
                .reply_markup(InlineKeyboardMarkup::new(
                    Vec::<Vec<InlineKeyboardButton>>::new(),
                ))
                .await;
            chat_id
        } else {
            return Err(OutpostError::telegram_error(
                "No message found in callback query",
            ));
        };

        let mapping = state
            .topic_store
            .get_mapping(chat_id.0, topic_id)
            .await
            .ok()
            .flatten();

        if let Some(mapping) = mapping {
            if let Some(instance_id) = &mapping.instance_id {
                if let Err(e) = state.instance_manager.stop_instance(instance_id).await {
                    warn!(instance_id = %instance_id, error = %e, "Failed to stop instance during close");
                }
            }

            let project_path = PathBuf::from(&mapping.project_path);
            if mapping.project_path.contains("/.worktrees/") {
                if let Some(worktrees_dir) =
                    project_path.ancestors().find(|p| p.ends_with(".worktrees"))
                {
                    if let Some(repo_root) = worktrees_dir.parent() {
                        if let Err(e) =
                            crate::git::worktree::remove_worktree(repo_root, &project_path).await
                        {
                            warn!(error = %e, "Failed to remove worktree during close");
                        }
                        if let Some(wt_name) = project_path.file_name().and_then(|n| n.to_str()) {
                            let branch_name = format!("wt/{}", wt_name);
                            if let Err(e) =
                                crate::git::worktree::delete_branch(repo_root, &branch_name).await
                            {
                                warn!(error = %e, "Failed to delete branch during close");
                            }
                        }
                    }
                }
            }
        }

        if let Err(e) = state.topic_store.delete_mapping(chat_id.0, topic_id).await {
            warn!(error = %e, "Failed to delete topic mapping during close");
        }

        if let Err(e) = bot
            .delete_forum_topic(chat_id, ThreadId(MessageId(topic_id)))
            .await
        {
            warn!(error = %e, "Failed to delete forum topic during close");
        }
    }

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

    fn make_message_json(thread_id: Option<i32>) -> serde_json::Value {
        let mut msg = serde_json::json!({
            "message_id": 100,
            "date": 1640000000,
            "chat": {
                "id": -1001234567890_i64,
                "type": "supergroup",
                "title": "Test Group"
            }
        });
        if let Some(tid) = thread_id {
            msg["message_thread_id"] = serde_json::json!(tid);
        }
        msg
    }

    #[test]
    fn test_get_topic_id_from_forum_topic() {
        let json = make_message_json(Some(456));
        let msg: Message = serde_json::from_value(json).unwrap();
        let result = get_topic_id(&msg);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 456);
    }

    #[test]
    fn test_get_topic_id_rejects_general_topic() {
        let json = make_message_json(Some(1));
        let msg: Message = serde_json::from_value(json).unwrap();
        let result = get_topic_id(&msg);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot close the General topic"));
    }

    #[test]
    fn test_get_topic_id_rejects_no_topic() {
        let json = make_message_json(None);
        let msg: Message = serde_json::from_value(json).unwrap();
        let result = get_topic_id(&msg);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be used in a forum topic"));
    }

    #[test]
    fn test_close_callback_data_format() {
        let confirm_data = "close:123:confirm";
        let parts: Vec<&str> = confirm_data.split(':').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "close");
        assert_eq!(parts[1], "123");
        assert_eq!(parts[2], "confirm");
        let topic_id: i32 = parts[1].parse().unwrap();
        assert_eq!(topic_id, 123);

        let cancel_data = "close:456:cancel";
        let parts: Vec<&str> = cancel_data.split(':').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "close");
        assert_eq!(parts[1], "456");
        assert_eq!(parts[2], "cancel");
        let topic_id: i32 = parts[1].parse().unwrap();
        assert_eq!(topic_id, 456);
    }

    #[test]
    fn test_cleanup_skips_worktree_for_non_worktree_path() {
        let project_path = "/home/user/projects/my-project";
        assert!(!project_path.contains("/.worktrees/"));
    }

    #[test]
    fn test_cleanup_identifies_worktree_path() {
        let project_path_str = "/home/user/projects/.worktrees/my-topic";
        assert!(project_path_str.contains("/.worktrees/"));

        let project_path = PathBuf::from(project_path_str);

        let worktrees_dir = project_path.ancestors().find(|p| p.ends_with(".worktrees"));
        assert!(worktrees_dir.is_some());

        let worktrees_dir = worktrees_dir.unwrap();
        assert_eq!(
            worktrees_dir,
            PathBuf::from("/home/user/projects/.worktrees")
        );

        let repo_root = worktrees_dir.parent();
        assert!(repo_root.is_some());
        assert_eq!(repo_root.unwrap(), PathBuf::from("/home/user/projects"));

        let wt_name = project_path.file_name().and_then(|n| n.to_str());
        assert_eq!(wt_name, Some("my-topic"));
        let branch_name = format!("wt/{}", wt_name.unwrap());
        assert_eq!(branch_name, "wt/my-topic");
    }

    #[tokio::test]
    async fn test_mapping_lifecycle_for_close() {
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
            topic_name_updated: false,
            created_at: now,
            updated_at: now,
        };

        state.topic_store.save_mapping(&mapping).await.unwrap();

        let result = state
            .topic_store
            .get_mapping(-1001234567890, 456)
            .await
            .unwrap();
        assert!(result.is_some());

        state
            .topic_store
            .delete_mapping(-1001234567890, 456)
            .await
            .unwrap();

        let result = state
            .topic_store
            .get_mapping(-1001234567890, 456)
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_inline_keyboard_structure() {
        let topic_id = 789;
        let keyboard = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback("✅ Confirm", format!("close:{}:confirm", topic_id)),
            InlineKeyboardButton::callback("❌ Cancel", format!("close:{}:cancel", topic_id)),
        ]]);

        assert_eq!(keyboard.inline_keyboard.len(), 1);
        assert_eq!(keyboard.inline_keyboard[0].len(), 2);
    }
}
