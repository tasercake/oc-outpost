//! /sessions command handler
//!
//! Lists all active OpenCode sessions:
//! - Managed instances (spawned by InstanceManager)
//! - Discovered instances (found via process discovery)
//! - External instances (registered via API - future)

use crate::bot::{BotState, Command};
use crate::opencode::Discovery;
use crate::types::error::Result;
use std::path::Path;
use std::sync::Arc;
use teloxide::prelude::*;
use tracing::debug;

const MAX_SESSIONS_PER_PAGE: usize = 10;

/// Format session information for display.
///
/// Groups sessions by type (managed, discovered, external) and formats
/// with project name, path, session ID, and port/PID.
fn format_sessions(
    managed: Vec<ManagedSessionInfo>,
    discovered: Vec<DiscoveredSessionInfo>,
) -> String {
    let total = managed.len() + discovered.len();

    if total == 0 {
        return "No active sessions found.".to_string();
    }

    let mut output = format!("Active Sessions ({})\n\n", total);
    let mut shown = 0;

    for session in managed.iter() {
        if shown >= MAX_SESSIONS_PER_PAGE {
            break;
        }
        output.push_str(&format!(
            "{} (managed)\n{}\n{}\nPort: {}\n\n",
            session.name, session.path, session.session_id, session.port
        ));
        shown += 1;
    }

    for session in discovered.iter() {
        if shown >= MAX_SESSIONS_PER_PAGE {
            break;
        }
        output.push_str(&format!(
            "{} (discovered)\n{}\nPID: {}\n",
            session.name, session.path, session.pid
        ));
        if let Some(port) = session.port {
            output.push_str(&format!("Port: {}\n", port));
        }
        output.push('\n');
        shown += 1;
    }

    if total > MAX_SESSIONS_PER_PAGE {
        let remaining = total - MAX_SESSIONS_PER_PAGE;
        output.push_str(&format!("... and {} more\n", remaining));
    }

    output
}

/// Information about a managed session.
#[derive(Debug, Clone)]
struct ManagedSessionInfo {
    name: String,
    path: String,
    session_id: String,
    port: u16,
}

/// Information about a discovered session.
#[derive(Debug, Clone)]
struct DiscoveredSessionInfo {
    name: String,
    path: String,
    pid: u32,
    port: Option<u16>,
}

/// Extract managed session information from OrchestratorStore.
async fn get_managed_sessions(state: &BotState) -> Result<Vec<ManagedSessionInfo>> {
    let store = state.orchestrator_store.lock().await;
    let instances = store.get_all_instances().await.map_err(|e| {
        crate::types::error::OutpostError::database_error(format!(
            "Failed to list instances: {}",
            e
        ))
    })?;

    let sessions = instances
        .into_iter()
        .filter(|info| {
            matches!(
                info.state,
                crate::types::instance::InstanceState::Running
                    | crate::types::instance::InstanceState::Starting
            )
        })
        .map(|info| {
            let name = extract_project_name(&info.project_path);
            ManagedSessionInfo {
                name,
                path: info.project_path,
                session_id: info.id.to_string(),
                port: info.port,
            }
        })
        .collect();

    Ok(sessions)
}

/// Extract discovered session information from Discovery.
async fn get_discovered_sessions() -> Result<Vec<DiscoveredSessionInfo>> {
    let instances = Discovery::discover_all().await.map_err(|e| {
        crate::types::error::OutpostError::io_error(format!("Failed to discover instances: {}", e))
    })?;

    let sessions = instances
        .into_iter()
        .map(|inst| {
            let path = inst.working_dir.to_string_lossy().to_string();
            let name = extract_project_name(&path);

            DiscoveredSessionInfo {
                name,
                path,
                pid: inst.pid,
                port: inst.port,
            }
        })
        .collect();

    Ok(sessions)
}

/// Extract project name from path.
fn extract_project_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Handle /sessions command.
pub async fn handle_sessions(
    bot: Bot,
    msg: Message,
    _cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    debug!(
        chat_id = msg.chat.id.0,
        topic_id = ?msg.thread_id.map(|t| t.0 .0),
        sender_id = ?msg.from.as_ref().map(|u| u.id.0),
        "Handling /sessions"
    );
    let managed = get_managed_sessions(&state).await.unwrap_or_default();
    debug!(managed_count = managed.len(), "Managed sessions retrieved");
    let discovered = get_discovered_sessions().await.unwrap_or_default();
    debug!(
        discovered_count = discovered.len(),
        "Discovered sessions retrieved"
    );
    let total = managed.len() + discovered.len();
    debug!(total = total, "Total sessions to display");
    let output = format_sessions(managed, discovered);

    bot.send_message(msg.chat.id, output)
        .await
        .map_err(|e| crate::types::error::OutpostError::telegram_error(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_empty_list() {
        let output = format_sessions(vec![], vec![]);
        assert_eq!(output, "No active sessions found.");
    }

    #[test]
    fn test_format_single_managed() {
        let managed = vec![ManagedSessionInfo {
            name: "my-project".to_string(),
            path: "/home/user/my-project".to_string(),
            session_id: "ses_abc123".to_string(),
            port: 4100,
        }];

        let output = format_sessions(managed, vec![]);
        assert!(output.contains("Active Sessions (1)"));
        assert!(output.contains("my-project (managed)"));
        assert!(output.contains("/home/user/my-project"));
        assert!(output.contains("ses_abc123"));
        assert!(output.contains("Port: 4100"));
    }

    #[test]
    fn test_format_single_discovered() {
        let discovered = vec![DiscoveredSessionInfo {
            name: "other-project".to_string(),
            path: "/home/user/other-project".to_string(),
            pid: 12345,
            port: Some(4101),
        }];

        let output = format_sessions(vec![], discovered);
        assert!(output.contains("Active Sessions (1)"));
        assert!(output.contains("other-project (discovered)"));
        assert!(output.contains("/home/user/other-project"));
        assert!(output.contains("PID: 12345"));
        assert!(output.contains("Port: 4101"));
    }

    #[test]
    fn test_format_multiple_instances() {
        let managed = vec![
            ManagedSessionInfo {
                name: "project1".to_string(),
                path: "/home/user/project1".to_string(),
                session_id: "ses_abc123".to_string(),
                port: 4100,
            },
            ManagedSessionInfo {
                name: "project2".to_string(),
                path: "/home/user/project2".to_string(),
                session_id: "ses_def456".to_string(),
                port: 4101,
            },
        ];

        let output = format_sessions(managed, vec![]);
        assert!(output.contains("Active Sessions (2)"));
        assert!(output.contains("project1 (managed)"));
        assert!(output.contains("project2 (managed)"));
    }

    #[test]
    fn test_format_mixed_types() {
        let managed = vec![ManagedSessionInfo {
            name: "managed-proj".to_string(),
            path: "/home/user/managed-proj".to_string(),
            session_id: "ses_abc123".to_string(),
            port: 4100,
        }];

        let discovered = vec![DiscoveredSessionInfo {
            name: "discovered-proj".to_string(),
            path: "/home/user/discovered-proj".to_string(),
            pid: 12345,
            port: Some(4101),
        }];

        let output = format_sessions(managed, discovered);
        assert!(output.contains("Active Sessions (2)"));
        assert!(output.contains("managed-proj (managed)"));
        assert!(output.contains("discovered-proj (discovered)"));
    }

    #[test]
    fn test_pagination_many_instances() {
        let mut managed = Vec::new();
        for i in 0..15 {
            managed.push(ManagedSessionInfo {
                name: format!("project{}", i),
                path: format!("/home/user/project{}", i),
                session_id: format!("ses_{}", i),
                port: 4100 + i as u16,
            });
        }

        let output = format_sessions(managed, vec![]);
        assert!(output.contains("Active Sessions (15)"));
        assert!(output.contains("... and 5 more"));
        assert!(output.contains("project0"));
        assert!(output.contains("project9"));
        assert!(!output.contains("project10"));
    }

    #[test]
    fn test_extract_project_name() {
        assert_eq!(extract_project_name("/home/user/my-project"), "my-project");
        assert_eq!(extract_project_name("/my-project"), "my-project");
        assert_eq!(extract_project_name("my-project"), "my-project");
        assert_eq!(extract_project_name(""), "unknown");
    }

    #[test]
    fn test_discovered_without_port() {
        let discovered = vec![DiscoveredSessionInfo {
            name: "no-port-proj".to_string(),
            path: "/home/user/no-port-proj".to_string(),
            pid: 12345,
            port: None,
        }];

        let output = format_sessions(vec![], discovered);
        assert!(output.contains("no-port-proj (discovered)"));
        assert!(output.contains("PID: 12345"));
        assert!(!output.contains("Port:"));
    }
}
