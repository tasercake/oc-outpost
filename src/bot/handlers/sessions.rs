//! /sessions command handler

use crate::bot::{BotState, Command};
use crate::types::error::Result;
use std::path::Path;
use std::sync::Arc;
use teloxide::prelude::*;
use tracing::debug;

const MAX_SESSIONS_PER_PAGE: usize = 10;

#[derive(Debug, Clone)]
struct SessionInfo {
    name: String,
    path: String,
    session_id: String,
    port: u16,
}

fn format_sessions(sessions: &[SessionInfo]) -> String {
    if sessions.is_empty() {
        return "No active sessions found.".to_string();
    }

    let mut output = format!("Active Sessions ({})\n\n", sessions.len());

    for session in sessions.iter().take(MAX_SESSIONS_PER_PAGE) {
        output.push_str(&format!(
            "{}\n{}\n{}\nPort: {}\n\n",
            session.name, session.path, session.session_id, session.port
        ));
    }

    if sessions.len() > MAX_SESSIONS_PER_PAGE {
        let remaining = sessions.len() - MAX_SESSIONS_PER_PAGE;
        output.push_str(&format!("... and {} more\n", remaining));
    }

    output
}

async fn get_sessions(state: &BotState) -> Result<Vec<SessionInfo>> {
    let instances = state
        .orchestrator_store
        .get_all_instances()
        .await
        .map_err(|e| {
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
            SessionInfo {
                name,
                path: info.project_path,
                session_id: info.id.to_string(),
                port: info.port,
            }
        })
        .collect();

    Ok(sessions)
}

fn extract_project_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

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
    let sessions = get_sessions(&state).await.unwrap_or_default();
    debug!(count = sessions.len(), "Sessions retrieved");
    let output = format_sessions(&sessions);

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
        let output = format_sessions(&[]);
        assert_eq!(output, "No active sessions found.");
    }

    #[test]
    fn test_format_single_session() {
        let sessions = vec![SessionInfo {
            name: "my-project".to_string(),
            path: "/home/user/my-project".to_string(),
            session_id: "ses_abc123".to_string(),
            port: 4100,
        }];

        let output = format_sessions(&sessions);
        assert!(output.contains("Active Sessions (1)"));
        assert!(output.contains("my-project"));
        assert!(output.contains("/home/user/my-project"));
        assert!(output.contains("ses_abc123"));
        assert!(output.contains("Port: 4100"));
    }

    #[test]
    fn test_format_multiple_sessions() {
        let sessions = vec![
            SessionInfo {
                name: "project1".to_string(),
                path: "/home/user/project1".to_string(),
                session_id: "ses_abc123".to_string(),
                port: 4100,
            },
            SessionInfo {
                name: "project2".to_string(),
                path: "/home/user/project2".to_string(),
                session_id: "ses_def456".to_string(),
                port: 4101,
            },
        ];

        let output = format_sessions(&sessions);
        assert!(output.contains("Active Sessions (2)"));
        assert!(output.contains("project1"));
        assert!(output.contains("project2"));
    }

    #[test]
    fn test_pagination_many_sessions() {
        let sessions: Vec<SessionInfo> = (0..15)
            .map(|i| SessionInfo {
                name: format!("project{}", i),
                path: format!("/home/user/project{}", i),
                session_id: format!("ses_{}", i),
                port: 4100 + i as u16,
            })
            .collect();

        let output = format_sessions(&sessions);
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
}
