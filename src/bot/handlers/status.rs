//! /status command handler

use crate::bot::{BotState, Command};
use crate::types::error::{OutpostError, Result};
use std::sync::Arc;
use teloxide::prelude::*;
use tracing::debug;

/// Format uptime from seconds to human-readable format (e.g., "2h 15m")
fn format_uptime(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

/// Format status output for display
fn format_status_output(
    total_count: usize,
    port_used: usize,
    port_total: usize,
    uptime_seconds: u64,
) -> String {
    let mut output = String::from("Orchestrator Status\n\n");

    output.push_str(&format!("Active Instances: {}\n", total_count));
    output.push('\n');
    output.push_str(&format!("Port Pool: {}/{} used\n", port_used, port_total));
    output.push_str(&format!("Uptime: {}\n", format_uptime(uptime_seconds)));
    output.push_str("Health: Healthy\n");

    output
}

/// Handle /status command
pub async fn handle_status(
    bot: Bot,
    msg: Message,
    _cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    debug!(
        chat_id = msg.chat.id.0,
        topic_id = ?msg.thread_id.map(|t| t.0 .0),
        sender_id = ?msg.from.as_ref().map(|u| u.id.0),
        "Handling /status"
    );
    let chat_id = msg.chat.id;

    // Get orchestrator store
    let all_instances = state
        .orchestrator_store
        .get_all_instances()
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?;

    // Count instances
    let total_count = all_instances.len();

    debug!(total_count = total_count, "Instance count");

    // Get port pool usage from InstanceManager
    let manager_status = state.instance_manager.get_status().await;
    debug!(
        available_ports = manager_status.available_ports,
        "Manager status fetched"
    );
    let port_total = state.config.opencode_port_pool_size as usize;
    let port_used = port_total - manager_status.available_ports;

    // Calculate uptime (from bot start time)
    let uptime_seconds = state.bot_start_time.elapsed().as_secs();

    // Format and send message
    let output = format_status_output(total_count, port_used, port_total, uptime_seconds);

    bot.send_message(chat_id, output)
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_uptime_hours_and_minutes() {
        let uptime = 8100; // 2h 15m
        let formatted = format_uptime(uptime);
        assert_eq!(formatted, "2h 15m");
    }

    #[test]
    fn test_format_uptime_only_minutes() {
        let uptime = 900; // 15m
        let formatted = format_uptime(uptime);
        assert_eq!(formatted, "15m");
    }

    #[test]
    fn test_format_uptime_zero() {
        let uptime = 0;
        let formatted = format_uptime(uptime);
        assert_eq!(formatted, "0m");
    }

    #[test]
    fn test_format_uptime_one_hour() {
        let uptime = 3600; // 1h 0m
        let formatted = format_uptime(uptime);
        assert_eq!(formatted, "1h 0m");
    }

    #[test]
    fn test_format_status_output_basic() {
        let output = format_status_output(3, 4, 100, 8100);

        assert!(output.contains("Orchestrator Status"));
        assert!(output.contains("Active Instances: 3"));
        assert!(output.contains("Port Pool: 4/100 used"));
        assert!(output.contains("Uptime: 2h 15m"));
        assert!(output.contains("Health: Healthy"));
    }

    #[test]
    fn test_format_status_output_no_instances() {
        let output = format_status_output(0, 0, 100, 300);

        assert!(output.contains("Active Instances: 0"));
        assert!(output.contains("Port Pool: 0/100 used"));
        assert!(output.contains("Uptime: 5m"));
    }

    #[test]
    fn test_format_status_output_all_active() {
        let output = format_status_output(10, 10, 100, 3600);

        assert!(output.contains("Active Instances: 10"));
        assert!(output.contains("Port Pool: 10/100 used"));
        assert!(output.contains("Uptime: 1h 0m"));
    }

    #[test]
    fn test_format_status_output_mixed() {
        let output = format_status_output(5, 8, 100, 7200);

        assert!(output.contains("Active Instances: 5"));
        assert!(output.contains("Port Pool: 8/100 used"));
        assert!(output.contains("Uptime: 2h 0m"));
    }
}
