//! /session command handler
//!
//! Displays session information for the current topic:
//! - Instance type (Managed, Discovered, External)
//! - Session ID
//! - Project path
//! - Port
//! - Status
//! - Streaming status
//! - Creation timestamp

use crate::bot::{BotState, Command};
use crate::types::error::{OutpostError, Result};
use crate::types::forum::TopicMapping;
use crate::types::instance::InstanceInfo;
use std::sync::Arc;
use teloxide::prelude::*;
use tracing::debug;

/// Extract topic_id from message, ensuring it's not the General topic
fn get_topic_id(msg: &Message) -> Result<i32> {
    let thread_id = msg.thread_id.ok_or_else(|| {
        OutpostError::telegram_error("This command must be used in a forum topic")
    })?;

    // General topic has ThreadId(MessageId(1))
    if thread_id.0 .0 == 1 {
        return Err(OutpostError::telegram_error(
            "This command must be used in a forum topic",
        ));
    }

    Ok(thread_id.0 .0)
}

/// Format timestamp (Unix seconds) to readable format
fn format_timestamp(timestamp: i64) -> String {
    // Convert Unix timestamp to a basic date format
    // This is a simple implementation without external dependencies
    let days_since_epoch = timestamp / 86400;
    let seconds_today = timestamp % 86400;

    let hours = seconds_today / 3600;
    let minutes = (seconds_today % 3600) / 60;
    let seconds = seconds_today % 60;

    // Approximate year/month/day calculation (simplified, not accounting for leap years precisely)
    let mut year = 1970;
    let mut remaining_days = days_since_epoch;

    loop {
        let days_in_year = if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
            366
        } else {
            365
        };

        if remaining_days < days_in_year {
            break;
        }

        remaining_days -= days_in_year;
        year += 1;
    }

    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let days_in_months = if is_leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    let mut day = remaining_days + 1;

    for &days_in_month in &days_in_months {
        if day <= days_in_month as i64 {
            break;
        }
        day -= days_in_month as i64;
        month += 1;
    }

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hours, minutes, seconds
    )
}

/// Format session information for display
fn format_session_info(mapping: &TopicMapping, instance: Option<&InstanceInfo>) -> String {
    let mut output = String::from("Session Info\n\n");

    // Type
    if let Some(inst) = instance {
        output.push_str(&format!("Type: {:?}\n", inst.instance_type));
        output.push_str(&format!("Status: {:?}\n", inst.state));
        output.push_str(&format!("Port: {}\n", inst.port));
    } else {
        output.push_str("Type: (not available)\n");
        output.push_str("Status: (not available)\n");
        output.push_str("Port: (not available)\n");
    }

    // Session ID
    if let Some(session_id) = &mapping.session_id {
        output.push_str(&format!("Session: {}\n", session_id));
    } else {
        output.push_str("Session: (not available)\n");
    }

    // Project path
    output.push_str(&format!("Project: {}\n", mapping.project_path));

    // Streaming
    output.push_str(&format!(
        "Streaming: {}\n",
        if mapping.streaming_enabled {
            "ON"
        } else {
            "OFF"
        }
    ));

    // Created timestamp
    let created = format_timestamp(mapping.created_at);
    output.push_str(&format!("Created: {}\n", created));

    output
}

/// Handle /session command
pub async fn handle_session(
    bot: Bot,
    msg: Message,
    _cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    debug!(
        chat_id = msg.chat.id.0,
        topic_id = ?msg.thread_id.map(|t| t.0 .0),
        sender_id = ?msg.from.as_ref().map(|u| u.id.0),
        "Handling /session"
    );
    let topic_id = get_topic_id(&msg)?;
    let chat_id = msg.chat.id;

    // Get topic mapping
    let topic_store = state.topic_store.lock().await;
    let mapping = topic_store
        .get_mapping(topic_id)
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?
        .ok_or_else(|| OutpostError::telegram_error("No active connection in this topic"))?;
    drop(topic_store);
    debug!(topic_id = topic_id, session_id = ?mapping.session_id, instance_id = ?mapping.instance_id, "Mapping found for session info");

    // Get instance info if available
    let instance = if let Some(instance_id) = &mapping.instance_id {
        let store = state.orchestrator_store.lock().await;
        let inst = store
            .get_instance(instance_id)
            .await
            .map_err(|e| OutpostError::database_error(e.to_string()))?;
        drop(store);
        inst
    } else {
        None
    };

    debug!(
        instance_found = instance.is_some(),
        "Instance lookup result"
    );

    // Format and send message
    let output = format_session_info(&mapping, instance.as_ref());
    bot.send_message(chat_id, output)
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::instance::{InstanceState, InstanceType};

    #[test]
    fn test_format_with_all_fields() {
        let mapping = TopicMapping {
            topic_id: 123,
            chat_id: -1001234567890,
            project_path: "/home/user/my-project".to_string(),
            session_id: Some("ses_abc123456".to_string()),
            instance_id: Some("inst_001".to_string()),
            streaming_enabled: true,
            topic_name_updated: false,
            created_at: 1640000000,
            updated_at: 1640000100,
        };

        let instance = InstanceInfo {
            id: "inst_001".to_string(),
            state: InstanceState::Running,
            instance_type: InstanceType::Managed,
            project_path: "/home/user/my-project".to_string(),
            port: 4101,
            pid: Some(12345),
            container_id: None,
            started_at: Some(1640000000),
            stopped_at: None,
        };

        let output = format_session_info(&mapping, Some(&instance));

        assert!(output.contains("Session Info"));
        assert!(output.contains("Type: Managed"));
        assert!(output.contains("Status: Running"));
        assert!(output.contains("Port: 4101"));
        assert!(output.contains("Session: ses_abc123456"));
        assert!(output.contains("Project: /home/user/my-project"));
        assert!(output.contains("Streaming: ON"));
        assert!(output.contains("Created: 2021-12-20"));
    }

    #[test]
    fn test_format_with_missing_fields() {
        let mapping = TopicMapping {
            topic_id: 123,
            chat_id: -1001234567890,
            project_path: "/home/user/my-project".to_string(),
            session_id: None,
            instance_id: None,
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: 1640000000,
            updated_at: 1640000100,
        };

        let output = format_session_info(&mapping, None);

        assert!(output.contains("Session Info"));
        assert!(output.contains("Type: (not available)"));
        assert!(output.contains("Status: (not available)"));
        assert!(output.contains("Port: (not available)"));
        assert!(output.contains("Session: (not available)"));
        assert!(output.contains("Project: /home/user/my-project"));
        assert!(output.contains("Streaming: OFF"));
        assert!(output.contains("Created: 2021-12-20"));
    }

    #[test]
    fn test_format_with_partial_fields() {
        let mapping = TopicMapping {
            topic_id: 456,
            chat_id: -1009876543210,
            project_path: "/another/path".to_string(),
            session_id: Some("ses_xyz789".to_string()),
            instance_id: Some("inst_002".to_string()),
            streaming_enabled: true,
            topic_name_updated: true,
            created_at: 1650000000,
            updated_at: 1650000200,
        };

        let instance = InstanceInfo {
            id: "inst_002".to_string(),
            state: InstanceState::Stopped,
            instance_type: InstanceType::Discovered,
            project_path: "/another/path".to_string(),
            port: 4102,
            pid: None,
            container_id: None,
            started_at: None,
            stopped_at: Some(1650000200),
        };

        let output = format_session_info(&mapping, Some(&instance));

        assert!(output.contains("Type: Discovered"));
        assert!(output.contains("Status: Stopped"));
        assert!(output.contains("Port: 4102"));
        assert!(output.contains("Session: ses_xyz789"));
        assert!(output.contains("Streaming: ON"));
    }

    #[test]
    fn test_timestamp_formatting() {
        let mapping = TopicMapping {
            topic_id: 789,
            chat_id: -1001111111111,
            project_path: "/test/path".to_string(),
            session_id: Some("ses_test".to_string()),
            instance_id: None,
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: 1609459200, // 2021-01-01 00:00:00 UTC
            updated_at: 1609459200,
        };

        let output = format_session_info(&mapping, None);

        assert!(output.contains("Created: 2021-01-01"));
    }

    #[test]
    fn test_format_external_instance() {
        let mapping = TopicMapping {
            topic_id: 999,
            chat_id: -1002222222222,
            project_path: "/external/project".to_string(),
            session_id: Some("ses_ext123".to_string()),
            instance_id: Some("inst_ext".to_string()),
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: 1660000000,
            updated_at: 1660000300,
        };

        let instance = InstanceInfo {
            id: "inst_ext".to_string(),
            state: InstanceState::Running,
            instance_type: InstanceType::External,
            project_path: "/external/project".to_string(),
            port: 4103,
            pid: None,
            container_id: None,
            started_at: None,
            stopped_at: None,
        };

        let output = format_session_info(&mapping, Some(&instance));

        assert!(output.contains("Type: External"));
        assert!(output.contains("Status: Running"));
        assert!(output.contains("Port: 4103"));
        assert!(output.contains("Session: ses_ext123"));
        assert!(output.contains("Streaming: OFF"));
    }
}
