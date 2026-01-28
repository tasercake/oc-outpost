use crate::bot::BotState;
use crate::opencode::OpenCodeClient;
use crate::types::error::{OutpostError, Result};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

/// Format permission request message
fn format_permission_message(description: &str) -> String {
    format!(
        "Permission Request\n\nOpenCode wants to:\n[{}]",
        description
    )
}

/// Create inline keyboard with Allow/Deny buttons
fn create_inline_keyboard(session_id: &str, permission_id: &str) -> InlineKeyboardMarkup {
    let allow_data = format!("perm:{}:{}:allow", session_id, permission_id);
    let deny_data = format!("perm:{}:{}:deny", session_id, permission_id);

    InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("Allow", allow_data),
        InlineKeyboardButton::callback("Deny", deny_data),
    ]])
}

/// Parse callback data format: perm:{session_id}:{permission_id}:{allow|deny}
fn parse_callback_data(data: &str) -> Result<(String, String, String)> {
    let parts: Vec<&str> = data.split(':').collect();
    if parts.len() != 4 || parts[0] != "perm" {
        return Err(OutpostError::telegram_error("Invalid callback data format"));
    }

    Ok((
        parts[1].to_string(),
        parts[2].to_string(),
        parts[3].to_string(),
    ))
}

/// Handle permission request from OpenCode
#[allow(dead_code)]
pub async fn handle_permission_request(
    bot: Bot,
    chat_id: ChatId,
    thread_id: i32,
    session_id: &str,
    permission_id: &str,
    description: &str,
) -> Result<()> {
    let message = format_permission_message(description);
    let keyboard = create_inline_keyboard(session_id, permission_id);

    bot.send_message(chat_id, message)
        .message_thread_id(teloxide::types::ThreadId(teloxide::types::MessageId(
            thread_id,
        )))
        .reply_markup(keyboard)
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;

    Ok(())
}

/// Handle permission callback (Allow/Deny button click)
#[allow(dead_code)]
pub async fn handle_permission_callback(
    bot: Bot,
    q: CallbackQuery,
    state: Arc<BotState>,
) -> Result<()> {
    let data = q
        .data
        .ok_or_else(|| OutpostError::telegram_error("No callback data"))?;

    let (session_id, permission_id, action) = parse_callback_data(&data)?;

    let allow = action == "allow";
    let client = OpenCodeClient::new(&format!(
        "http://localhost:{}",
        state.config.opencode_port_start
    ));
    client
        .reply_permission(&session_id, &permission_id, allow)
        .await
        .map_err(|e| OutpostError::opencode_api_error(e.to_string()))?;

    if let Some(message) = q.message {
        let result_text = if allow { "✅ Allowed" } else { "❌ Denied" };
        let chat_id = message.chat().id;
        let message_id = message.id();
        bot.edit_message_text(chat_id, message_id, result_text)
            .await
            .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
    }

    bot.answer_callback_query(q.id)
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_permission_message() {
        let msg = format_permission_message("Delete file: src/old_module.rs");
        assert!(msg.contains("Permission Request"));
        assert!(msg.contains("OpenCode wants to:"));
        assert!(msg.contains("Delete file: src/old_module.rs"));
    }

    #[test]
    fn test_create_inline_keyboard() {
        let keyboard = create_inline_keyboard("ses_123", "perm_456");
        assert_eq!(keyboard.inline_keyboard.len(), 1);
        assert_eq!(keyboard.inline_keyboard[0].len(), 2);
    }

    #[test]
    fn test_parse_callback_data_allow() {
        let data = "perm:ses_123:perm_456:allow";
        let (session, perm, action) = parse_callback_data(data).unwrap();
        assert_eq!(session, "ses_123");
        assert_eq!(perm, "perm_456");
        assert_eq!(action, "allow");
    }

    #[test]
    fn test_parse_callback_data_deny() {
        let data = "perm:ses_123:perm_456:deny";
        let (session, perm, action) = parse_callback_data(data).unwrap();
        assert_eq!(session, "ses_123");
        assert_eq!(perm, "perm_456");
        assert_eq!(action, "deny");
    }

    #[test]
    fn test_parse_callback_data_invalid_format() {
        let data = "invalid:data";
        let result = parse_callback_data(data);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid callback data format"));
    }

    #[test]
    fn test_parse_callback_data_wrong_prefix() {
        let data = "wrong:ses_123:perm_456:allow";
        let result = parse_callback_data(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_callback_data_missing_parts() {
        let data = "perm:ses_123:perm_456";
        let result = parse_callback_data(data);
        assert!(result.is_err());
    }
}
