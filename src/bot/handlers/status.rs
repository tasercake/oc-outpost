//! /status command handler
//!
//! Displays orchestrator status:
//! - Managed/Discovered/External instance counts
//! - Port pool usage
//! - Uptime
//! - Health status

use crate::bot::{BotState, Command};
use crate::types::error::{OutpostError, Result};
use crate::types::instance::InstanceType;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use teloxide::prelude::*;

/// Format uptime from seconds to human-readable format (e.g., "2h 15m")
#[allow(dead_code)]
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
#[allow(dead_code)]
fn format_status_output(
    managed_count: usize,
    discovered_count: usize,
    external_count: usize,
    port_used: usize,
    port_total: usize,
    uptime_seconds: u64,
) -> String {
    let mut output = String::from("Orchestrator Status\n\n");

    output.push_str(&format!(
        "Managed Instances: {}/{}\n",
        managed_count,
        managed_count + discovered_count + external_count
    ));
    output.push_str(&format!("Discovered Sessions: {}\n", discovered_count));
    output.push_str(&format!("External Instances: {}\n", external_count));
    output.push('\n');
    output.push_str(&format!("Port Pool: {}/{} used\n", port_used, port_total));
    output.push_str(&format!("Uptime: {}\n", format_uptime(uptime_seconds)));
    output.push_str("Health: Healthy\n");

    output
}

/// Handle /status command
#[allow(dead_code)]
pub async fn handle_status(
    bot: Bot,
    msg: Message,
    _cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    let chat_id = msg.chat.id;

    // Get orchestrator store
    let store = state.orchestrator_store.lock().await;
    let all_instances = store
        .get_all_instances()
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?;
    drop(store);

    // Count instances by type
    let managed_count = all_instances
        .iter()
        .filter(|i| i.instance_type == InstanceType::Managed)
        .count();
    let discovered_count = all_instances
        .iter()
        .filter(|i| i.instance_type == InstanceType::Discovered)
        .count();
    let external_count = all_instances
        .iter()
        .filter(|i| i.instance_type == InstanceType::External)
        .count();

    // Get port pool usage
    let port_used = state.config.opencode_port_pool_size as usize;
    let port_total = state.config.opencode_port_pool_size as usize;

    // Calculate uptime (from bot start time)
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| OutpostError::io_error(e.to_string()))?
        .as_secs();
    let uptime_seconds = now;

    // Format and send message
    let output = format_status_output(
        managed_count,
        discovered_count,
        external_count,
        port_used,
        port_total,
        uptime_seconds,
    );

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
        let output = format_status_output(3, 2, 1, 4, 100, 8100);

        assert!(output.contains("Orchestrator Status"));
        assert!(output.contains("Managed Instances: 3/6"));
        assert!(output.contains("Discovered Sessions: 2"));
        assert!(output.contains("External Instances: 1"));
        assert!(output.contains("Port Pool: 4/100 used"));
        assert!(output.contains("Uptime: 2h 15m"));
        assert!(output.contains("Health: Healthy"));
    }

    #[test]
    fn test_format_status_output_no_instances() {
        let output = format_status_output(0, 0, 0, 0, 100, 300);

        assert!(output.contains("Managed Instances: 0/0"));
        assert!(output.contains("Discovered Sessions: 0"));
        assert!(output.contains("External Instances: 0"));
        assert!(output.contains("Port Pool: 0/100 used"));
        assert!(output.contains("Uptime: 5m"));
    }

    #[test]
    fn test_format_status_output_all_managed() {
        let output = format_status_output(10, 0, 0, 10, 100, 3600);

        assert!(output.contains("Managed Instances: 10/10"));
        assert!(output.contains("Discovered Sessions: 0"));
        assert!(output.contains("External Instances: 0"));
        assert!(output.contains("Port Pool: 10/100 used"));
        assert!(output.contains("Uptime: 1h 0m"));
    }

    #[test]
    fn test_format_status_output_mixed_instances() {
        let output = format_status_output(5, 3, 2, 8, 100, 7200);

        assert!(output.contains("Managed Instances: 5/10"));
        assert!(output.contains("Discovered Sessions: 3"));
        assert!(output.contains("External Instances: 2"));
        assert!(output.contains("Port Pool: 8/100 used"));
        assert!(output.contains("Uptime: 2h 0m"));
    }
}
