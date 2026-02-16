//! /help command handler
//!
//! Context-aware help that shows different commands based on topic:
//! - General topic: All commands
//! - Forum topics: Only topic-relevant commands

use crate::bot::{BotState, Command};
use crate::types::error::Result;
use std::sync::Arc;
use teloxide::prelude::*;
use tracing::debug;

/// Format help text for General topic (all commands).
fn format_general_help() -> String {
    "OpenCode Telegram Bot\n\n\
     Commands:\n\
     /new <name> - Create new project topic\n\
     /sessions - List active sessions\n\
     /projects - List available projects\n\
     /status - Show bot status\n\
     /help - This help\n\n\
     In a topic:\n\
     /session - Show session info\n\
     /close - Close topic and stop instance"
        .to_string()
}

/// Format help text for forum topics (topic-relevant commands only).
fn format_topic_help() -> String {
    "Topic Commands:\n\n\
     /session - Show session info\n\
     /close - Close topic and stop instance\n\n\
     Use /help in General topic for all commands."
        .to_string()
}

/// Handle /help command.
///
/// Shows context-aware help:
/// - In General topic: All commands
/// - In forum topics: Only topic-relevant commands
pub async fn handle_help(
    bot: Bot,
    msg: Message,
    _cmd: Command,
    _state: Arc<BotState>,
) -> Result<()> {
    debug!(
        chat_id = msg.chat.id.0,
        topic_id = ?msg.thread_id.map(|t| t.0 .0),
        sender_id = ?msg.from.as_ref().map(|u| u.id.0),
        "Handling /help"
    );
    // Detect if we're in a forum topic
    let is_topic = msg.thread_id.is_some();
    debug!(is_topic = is_topic, "Help context detection");

    let help_text = if is_topic {
        format_topic_help()
    } else {
        format_general_help()
    };

    bot.send_message(msg.chat.id, help_text)
        .await
        .map_err(|e| crate::types::error::OutpostError::telegram_error(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_general_help() {
        let help = format_general_help();

        // Verify header
        assert!(help.contains("OpenCode Telegram Bot"));

        // Verify commands section
        assert!(help.contains("Commands:"));
        assert!(help.contains("/new <name> - Create new project topic"));
        assert!(help.contains("/sessions - List active sessions"));
        assert!(help.contains("/projects - List available projects"));
        assert!(help.contains("/status - Show bot status"));
        assert!(help.contains("/help - This help"));

        // Verify topic commands section
        assert!(help.contains("In a topic:"));
        assert!(help.contains("/session - Show session info"));
        assert!(help.contains("/close - Close topic and stop instance"));

        // Verify removed commands are absent
        assert!(!help.contains("/connect"));
        assert!(!help.contains("/clear"));
        assert!(!help.contains("/link"));
        assert!(!help.contains("/stream"));
    }

    #[test]
    fn test_format_topic_help() {
        let help = format_topic_help();

        // Verify header
        assert!(help.contains("Topic Commands:"));

        // Verify topic commands
        assert!(help.contains("/session - Show session info"));
        assert!(help.contains("/close - Close topic and stop instance"));

        // Verify reference to general help
        assert!(help.contains("Use /help in General topic for all commands."));

        // Verify general commands are NOT in topic help
        assert!(!help.contains("/new"));
        assert!(!help.contains("/sessions"));
        assert!(!help.contains("/projects"));
        assert!(!help.contains("/status"));

        // Verify removed commands are absent
        assert!(!help.contains("/connect"));
        assert!(!help.contains("/clear"));
        assert!(!help.contains("/link"));
        assert!(!help.contains("/stream"));
    }

    #[test]
    fn test_help_formatting_consistency() {
        let general = format_general_help();
        let topic = format_topic_help();

        // Both should be non-empty
        assert!(!general.is_empty());
        assert!(!topic.is_empty());

        // General should be longer (more commands)
        assert!(general.len() > topic.len());

        // Both should contain /help or reference to it
        assert!(general.contains("/help") || general.contains("help"));
        assert!(topic.contains("help"));
    }
}
