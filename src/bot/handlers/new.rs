use crate::bot::{BotState, Command};
use crate::git::worktree::{create_worktree, is_git_repo, sanitize_branch_name};
use crate::types::error::{OutpostError, Result};
use crate::types::forum::TopicMapping;
use std::sync::Arc;
use teloxide::prelude::*;
use tracing::debug;

/// Validate project name according to rules:
/// - Length: 1-50 characters
/// - Allowed: alphanumeric, dash, underscore
/// - No special chars, no spaces
fn validate_project_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(OutpostError::config_error("Project name cannot be empty"));
    }

    if name.len() > 50 {
        return Err(OutpostError::config_error(
            "Project name must be 50 characters or less",
        ));
    }

    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(OutpostError::config_error(
            "Project name must contain only alphanumeric characters, dashes, and underscores",
        ));
    }

    Ok(())
}

/// Check if message is in General topic (thread_id is None or ThreadId(MessageId(1)))
fn is_general_topic(msg: &Message) -> bool {
    msg.thread_id.is_none() || (msg.thread_id.map(|id| id.0) == Some(teloxide::types::MessageId(1)))
}

/// Handle /new command - create new project and session
///
/// Steps:
/// 1. Extract and validate project name
/// 2. Check if General topic (reject if HANDLE_GENERAL_TOPIC=false)
/// 3. Resolve existing project directory and optional worktree
/// 4. Create forum topic
/// 5. Spawn OpenCode instance via InstanceManager
/// 6. Create topic mapping in TopicStore
/// 7. Send confirmation message
pub async fn handle_new(bot: Bot, msg: Message, cmd: Command, state: Arc<BotState>) -> Result<()> {
    debug!(
        chat_id = msg.chat.id.0,
        topic_id = ?msg.thread_id.map(|t| t.0 .0),
        sender_id = ?msg.from.as_ref().map(|u| u.id.0),
        "Handling /new"
    );

    // Extract project name from command
    let name = match cmd {
        Command::New(n) => n,
        _ => return Err(OutpostError::config_error("Invalid command type")),
    };
    debug!(name = %name, "Project name extracted from command");

    // Validate project name
    validate_project_name(&name)?;
    debug!(name = %name, "Project name validated");

    // Check if in General topic
    debug!(
        is_general = is_general_topic(&msg),
        handle_general = state.config.handle_general_topic,
        "General topic check"
    );
    if is_general_topic(&msg) && !state.config.handle_general_topic {
        bot.send_message(
            msg.chat.id,
            "Cannot create projects in General topic. Please create a forum topic first.",
        )
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
        return Ok(());
    }

    let project_path = state.config.project_base_path.join(&name);
    debug!(project_path = %project_path.display(), "Resolved project path");

    debug!(project_path = %project_path.display(), exists = project_path.exists(), "Project directory existence check");
    if !project_path.is_dir() {
        bot.send_message(
            msg.chat.id,
            format!(
                "Directory '{}' not found under {}. Use /projects to see available directories.",
                name,
                state.config.project_base_path.display()
            ),
        )
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
        return Ok(());
    }

    let mut worktree_branch = None;
    let effective_project_path = if is_git_repo(&project_path) {
        let sanitized = sanitize_branch_name(&name);
        worktree_branch = Some(format!("wt/{}", sanitized));
        match create_worktree(&project_path, &name, &state.config.project_base_path).await {
            Ok(path) => path,
            Err(e) => {
                bot.send_message(msg.chat.id, format!("Failed to create worktree: {}", e))
                    .await
                    .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
                return Ok(());
            }
        }
    } else {
        bot.send_message(
            msg.chat.id,
            format!(
                "⚠️ Note: '{}' is not a git repository. Mounting directly without worktree isolation.",
                name
            ),
        )
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
        project_path.clone()
    };

    // Create forum topic
    let forum_topic = match bot.create_forum_topic(msg.chat.id, &name).await {
        Ok(topic) => topic,
        Err(e) => {
            bot.send_message(msg.chat.id, format!("Failed to create forum topic: {}", e))
                .await
                .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
            return Err(OutpostError::telegram_error(format!(
                "Failed to create forum topic: {}",
                e
            )));
        }
    };
    debug!(topic_id = forum_topic.thread_id.0.0, name = %name, "Forum topic created");

    // Spawn OpenCode instance via InstanceManager
    let topic_id = forum_topic.thread_id.0 .0;
    let _instance = state
        .instance_manager
        .get_or_create(&effective_project_path, topic_id)
        .await
        .map_err(|e| OutpostError::io_error(format!("Failed to spawn instance: {}", e)))?;

    let path_str = effective_project_path.to_string_lossy();
    let info = state
        .orchestrator_store
        .get_instance_by_path(&path_str)
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?
        .ok_or_else(|| OutpostError::io_error("Instance created but not found in store"))?;
    let instance_id = info.id;
    let port = info.port;
    debug!(instance_id = %instance_id, port = port, "Instance spawned for project");

    // Get timestamp
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| OutpostError::io_error(e.to_string()))?
        .as_secs() as i64;

    // Create and save TopicMapping with real instance_id
    let mapping = TopicMapping {
        topic_id,
        chat_id: msg.chat.id.0,
        project_path: effective_project_path.to_string_lossy().to_string(),
        session_id: None,
        instance_id: Some(instance_id.clone()),
        topic_name_updated: false,
        created_at: now,
        updated_at: now,
    };
    debug!(topic_id = mapping.topic_id, instance_id = ?mapping.instance_id, project_path = %mapping.project_path, "TopicMapping created, session_id will be set when OpenCode session starts");
    state
        .topic_store
        .save_mapping(&mapping)
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?;
    debug!(
        topic_id = mapping.topic_id,
        "TopicMapping saved to database"
    );

    // Send confirmation to the new topic with actual port
    let worktree_info = worktree_branch
        .as_ref()
        .map(|branch| format!("\n🌿 Worktree branch: {}", branch))
        .unwrap_or_default();
    let confirmation = format!(
        "🚀 Project '{}' created!\n\n\
         📁 Path: {}{}\n\
         🆔 Instance: {}\n\
         🔌 Port: {}\n\n\
         Send a message here to start your OpenCode session.",
        name,
        effective_project_path.display(),
        worktree_info,
        instance_id,
        port
    );
    bot.send_message(msg.chat.id, confirmation)
        .message_thread_id(teloxide::types::ThreadId(teloxide::types::MessageId(
            forum_topic.thread_id.0 .0,
        )))
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
    debug!(
        topic_id = forum_topic.thread_id.0 .0,
        "Confirmation message sent"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_project_name_valid() {
        assert!(validate_project_name("my-project").is_ok());
        assert!(validate_project_name("project_123").is_ok());
        assert!(validate_project_name("MyProject").is_ok());
        assert!(validate_project_name("a").is_ok());
        assert!(validate_project_name("project-with-dashes").is_ok());
        assert!(validate_project_name("project_with_underscores").is_ok());
        assert!(validate_project_name("123numeric").is_ok());
    }

    #[test]
    fn test_validate_project_name_empty() {
        let result = validate_project_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_project_name_too_long() {
        let long_name = "a".repeat(51);
        let result = validate_project_name(&long_name);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("50 characters or less"));
    }

    #[test]
    fn test_validate_project_name_invalid_chars() {
        assert!(validate_project_name("project with spaces").is_err());
        assert!(validate_project_name("project@special").is_err());
        assert!(validate_project_name("project#hash").is_err());
        assert!(validate_project_name("project!exclaim").is_err());
        assert!(validate_project_name("project/slash").is_err());
        assert!(validate_project_name("project\\backslash").is_err());
        assert!(validate_project_name("project.dot").is_err());
    }

    #[test]
    fn test_validate_project_name_boundary_length() {
        let exactly_50 = "a".repeat(50);
        assert!(validate_project_name(&exactly_50).is_ok());

        let exactly_51 = "a".repeat(51);
        assert!(validate_project_name(&exactly_51).is_err());
    }

    #[test]
    fn test_validate_project_name_with_dashes() {
        assert!(validate_project_name("my-project-name").is_ok());
        assert!(validate_project_name("project-123").is_ok());
        assert!(validate_project_name("a-b-c-d-e").is_ok());
    }

    #[test]
    fn test_validate_project_name_with_underscores() {
        assert!(validate_project_name("my_project_name").is_ok());
        assert!(validate_project_name("project_123").is_ok());
        assert!(validate_project_name("a_b_c_d_e").is_ok());
    }

    #[test]
    fn test_validate_project_name_mixed_valid_chars() {
        assert!(validate_project_name("My_Project-123").is_ok());
        assert!(validate_project_name("test_project-v2").is_ok());
        assert!(validate_project_name("ABC_123-xyz").is_ok());
    }

    #[test]
    fn test_validate_project_name_numeric_only() {
        assert!(validate_project_name("123456").is_ok());
        assert!(validate_project_name("42").is_ok());
    }

    #[test]
    fn test_validate_project_name_special_chars_rejected() {
        let special_chars = vec![
            ".", ",", "!", "@", "#", "$", "%", "^", "&", "*", "(", ")", "+", "=", "[", "]", "{",
            "}", "|", "\\", "/", "?", "<", ">", "~", "`", ":", ";", "\"", "'",
        ];

        for ch in special_chars {
            let name = format!("project{}name", ch);
            assert!(
                validate_project_name(&name).is_err(),
                "Should reject: {}",
                name
            );
        }
    }

    #[test]
    fn test_validate_project_name_whitespace_rejected() {
        assert!(validate_project_name("project name").is_err());
        assert!(validate_project_name("project\tname").is_err());
        assert!(validate_project_name("project\nname").is_err());
        assert!(validate_project_name(" project").is_err());
        assert!(validate_project_name("project ").is_err());
    }

    #[test]
    fn test_is_git_repo_integration() {
        let dir = TempDir::new().unwrap();
        assert!(!is_git_repo(dir.path()));
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        assert!(is_git_repo(dir.path()));
    }
}
