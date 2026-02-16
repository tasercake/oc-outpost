use crate::bot::BotState;
use crate::git::worktree::{create_worktree, is_git_repo, sanitize_branch_name};
use crate::types::error::{OutpostError, Result};
use crate::types::forum::TopicMapping;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, MessageId, ThreadId};
use tracing::{debug, info, warn};

pub async fn dispatch_callback(bot: Bot, q: CallbackQuery, state: Arc<BotState>) -> Result<()> {
    let data = match q.data.as_deref() {
        Some(d) => d,
        None => {
            let _ = bot.answer_callback_query(q.id).await;
            return Ok(());
        }
    };

    debug!(callback_data = %data, "Dispatching callback query");

    if data.starts_with("perm:") {
        crate::bot::handlers::permissions::handle_permission_callback(bot, q, state).await
    } else if data.starts_with("close:") {
        crate::bot::handlers::close::handle_close_callback(bot, q, state).await
    } else if data.starts_with("proj:") {
        handle_project_selection_callback(bot, q, state).await
    } else {
        warn!(callback_data = %data, "Unknown callback prefix");
        let _ = bot.answer_callback_query(q.id).text("Unknown action").await;
        Ok(())
    }
}

fn parse_project_callback_data(data: &str) -> Result<(i32, String)> {
    let parts: Vec<&str> = data.splitn(3, ':').collect();
    if parts.len() != 3 || parts[0] != "proj" {
        return Err(OutpostError::telegram_error(
            "Invalid project callback data format",
        ));
    }

    let topic_id: i32 = parts[1]
        .parse()
        .map_err(|_| OutpostError::telegram_error("Invalid topic ID in callback data"))?;

    let project_name = parts[2].to_string();
    if project_name.is_empty() {
        return Err(OutpostError::telegram_error(
            "Empty project name in callback data",
        ));
    }

    Ok((topic_id, project_name))
}

async fn handle_project_selection_callback(
    bot: Bot,
    q: CallbackQuery,
    state: Arc<BotState>,
) -> Result<()> {
    let data = q
        .data
        .as_deref()
        .ok_or_else(|| OutpostError::telegram_error("No callback data"))?;

    let (topic_id, project_name) = parse_project_callback_data(data)?;

    let chat_id = q
        .message
        .as_ref()
        .map(|m| m.chat().id)
        .ok_or_else(|| OutpostError::telegram_error("No message in callback"))?;

    debug!(
        topic_id = topic_id,
        project = %project_name,
        chat_id = chat_id.0,
        "Handling project selection callback"
    );

    let _ = bot
        .answer_callback_query(q.id.clone())
        .text(format!("Setting up '{}'...", project_name))
        .await;

    if let Some(ref message) = q.message {
        let _ = bot
            .edit_message_text(
                chat_id,
                message.id(),
                format!("Setting up project: {}...", project_name),
            )
            .reply_markup(InlineKeyboardMarkup::new(Vec::<
                Vec<teloxide::types::InlineKeyboardButton>,
            >::new()))
            .await;
    }

    let existing = state
        .topic_store
        .get_mapping(chat_id.0, topic_id)
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?;

    if existing.is_some() {
        bot.send_message(chat_id, "This topic is already set up!")
            .message_thread_id(ThreadId(MessageId(topic_id)))
            .await
            .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
        return Ok(());
    }

    let project_path = state.config.project_base_path.join(&project_name);
    if !project_path.is_dir() {
        bot.send_message(chat_id, format!("Directory '{}' not found.", project_name))
            .message_thread_id(ThreadId(MessageId(topic_id)))
            .await
            .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
        return Ok(());
    }

    let mut worktree_branch = None;
    let effective_project_path = if is_git_repo(&project_path) {
        let sanitized = sanitize_branch_name(&project_name);
        worktree_branch = Some(format!("wt/{}", sanitized));
        match create_worktree(
            &project_path,
            &project_name,
            &state.config.project_base_path,
        )
        .await
        {
            Ok(path) => path,
            Err(e) => {
                bot.send_message(chat_id, format!("Failed to create worktree: {}", e))
                    .message_thread_id(ThreadId(MessageId(topic_id)))
                    .await
                    .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
                return Ok(());
            }
        }
    } else {
        project_path.clone()
    };

    let instance_lock = state
        .instance_manager
        .get_or_create(&effective_project_path, topic_id)
        .await
        .map_err(|e| OutpostError::io_error(format!("Failed to spawn instance: {}", e)))?;

    let inst = instance_lock.lock().await;
    let instance_id = inst.id().to_string();
    let port = inst.port();
    drop(inst);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| OutpostError::io_error(e.to_string()))?
        .as_secs() as i64;

    let mapping = TopicMapping {
        topic_id,
        chat_id: chat_id.0,
        project_path: effective_project_path.to_string_lossy().to_string(),
        session_id: None,
        instance_id: Some(instance_id.clone()),
        topic_name_updated: false,
        created_at: now,
        updated_at: now,
    };

    state
        .topic_store
        .save_mapping(&mapping)
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?;

    let worktree_info = worktree_branch
        .as_ref()
        .map(|branch| format!("\nWorktree branch: {}", branch))
        .unwrap_or_default();

    let confirmation = format!(
        "Project '{}' linked to this topic!\n\n\
         Path: {}{}\n\
         Instance: {}\n\
         Port: {}\n\n\
         Send a message here to start your OpenCode session.",
        project_name,
        effective_project_path.display(),
        worktree_info,
        instance_id,
        port
    );

    bot.send_message(chat_id, confirmation)
        .message_thread_id(ThreadId(MessageId(topic_id)))
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;

    info!(
        topic_id = topic_id,
        project = %project_name,
        instance_id = %instance_id,
        "Project linked to topic via selection"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perm_prefix_detected() {
        let data = "perm:sess:pid:allow";
        assert!(data.starts_with("perm:"));
    }

    #[test]
    fn test_close_prefix_detected() {
        let data = "close:123:confirm";
        assert!(data.starts_with("close:"));
    }

    #[test]
    fn test_proj_prefix_detected() {
        let data = "proj:12345:my-project";
        assert!(data.starts_with("proj:"));
    }

    #[test]
    fn test_unknown_prefix_detected() {
        let data = "unknown:data";
        assert!(!data.starts_with("perm:"));
        assert!(!data.starts_with("close:"));
        assert!(!data.starts_with("proj:"));
    }

    #[test]
    fn test_parse_project_callback_data_valid() {
        let (topic_id, name) = parse_project_callback_data("proj:12345:my-project").unwrap();
        assert_eq!(topic_id, 12345);
        assert_eq!(name, "my-project");
    }

    #[test]
    fn test_parse_project_callback_data_with_dashes() {
        let (topic_id, name) = parse_project_callback_data("proj:999:name-with-dashes").unwrap();
        assert_eq!(topic_id, 999);
        assert_eq!(name, "name-with-dashes");
    }

    #[test]
    fn test_parse_project_callback_data_invalid_prefix() {
        let result = parse_project_callback_data("wrong:123:name");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_project_callback_data_missing_parts() {
        let result = parse_project_callback_data("proj:123");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_project_callback_data_empty_name() {
        let result = parse_project_callback_data("proj:123:");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_project_callback_data_invalid_topic_id() {
        let result = parse_project_callback_data("proj:notanumber:name");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_project_callback_data_large_topic_id() {
        let (topic_id, name) = parse_project_callback_data("proj:2147483647:project").unwrap();
        assert_eq!(topic_id, 2147483647);
        assert_eq!(name, "project");
    }
}
