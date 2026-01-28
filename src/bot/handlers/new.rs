use crate::bot::{BotState, Command};
use crate::types::error::{OutpostError, Result};
use std::sync::Arc;
use teloxide::prelude::*;

/// Validate project name according to rules:
/// - Length: 1-50 characters
/// - Allowed: alphanumeric, dash, underscore
/// - No special chars, no spaces
#[allow(dead_code)]
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
#[allow(dead_code)]
fn is_general_topic(msg: &Message) -> bool {
    msg.thread_id.is_none()
        || msg
            .thread_id
            .map(|id| id.0)
            .map_or(false, |raw_id| raw_id == teloxide::types::MessageId(1))
}

/// Handle /new command - create new project and session
///
/// Steps:
/// 1. Extract and validate project name
/// 2. Check if General topic (reject if HANDLE_GENERAL_TOPIC=false)
/// 3. Create project directory (if AUTO_CREATE_PROJECT_DIRS=true)
/// 4. Create forum topic
/// 5. Spawn OpenCode instance via InstanceManager
/// 6. Create topic mapping in TopicStore
/// 7. Send confirmation message
#[allow(dead_code)]
pub async fn handle_new(bot: Bot, msg: Message, cmd: Command, state: Arc<BotState>) -> Result<()> {
    // Extract project name from command
    let name = match cmd {
        Command::New(n) => n,
        _ => return Err(OutpostError::config_error("Invalid command type")),
    };

    // Validate project name
    validate_project_name(&name)?;

    // Check if in General topic
    if is_general_topic(&msg) && !state.config.handle_general_topic {
        bot.send_message(
            msg.chat.id,
            "Cannot create projects in General topic. Please create a forum topic first.",
        )
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
        return Ok(());
    }

    let _chat_id = msg.chat.id.0;
    let project_path = state.config.project_base_path.join(&name);

    // Check if directory already exists
    if project_path.exists() {
        bot.send_message(
            msg.chat.id,
            format!("Project directory '{}' already exists.", name),
        )
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
        return Ok(());
    }

    // Create project directory if enabled
    if state.config.auto_create_project_dirs {
        std::fs::create_dir_all(&project_path).map_err(|e| {
            OutpostError::io_error(format!(
                "Failed to create project directory '{}': {}",
                project_path.display(),
                e
            ))
        })?;
    }

    // Send success message (simplified - full implementation would create topic and instance)
    let message = format!(
        "✅ Project '{}' validated successfully\n\n\
        📁 Path: {}\n\n\
        (Full implementation will create forum topic and spawn OpenCode instance)",
        name,
        project_path.display()
    );

    bot.send_message(msg.chat.id, message)
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
