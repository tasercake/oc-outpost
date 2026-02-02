//! /projects command handler
//!
//! Lists all available project directories under PROJECT_BASE_PATH.
//! Displays directories in alphabetical order.

use crate::bot::{BotState, Command};
use crate::types::error::Result;
use std::sync::Arc;
use teloxide::prelude::*;
use tracing::debug;

/// Format project list for display
fn format_projects(dirs: Vec<String>, base_path: &str) -> String {
    if dirs.is_empty() {
        format!("No projects found in `{}`", base_path)
    } else {
        let list = dirs
            .iter()
            .map(|d| format!("• `{}`", d))
            .collect::<Vec<_>>()
            .join("\n");
        format!("📁 Available Projects\n\n{}", list)
    }
}

/// List project directories from the filesystem
fn list_project_dirs(base_path: &std::path::PathBuf) -> Vec<String> {
    match std::fs::read_dir(base_path) {
        Ok(entries) => {
            let mut dirs: Vec<String> = entries
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    if entry.file_type().ok()?.is_dir() {
                        Some(entry.file_name().to_string_lossy().to_string())
                    } else {
                        None
                    }
                })
                .collect();
            dirs.sort();
            dirs
        }
        Err(_) => vec![],
    }
}

/// Handle /projects command — list available project directories
pub async fn handle_projects(
    bot: Bot,
    msg: Message,
    _cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    debug!(
        chat_id = msg.chat.id.0,
        topic_id = ?msg.thread_id.map(|t| t.0 .0),
        sender_id = ?msg.from.as_ref().map(|u| u.id.0),
        "Handling /projects"
    );

    let base_path = &state.config.project_base_path;
    let dirs = list_project_dirs(base_path);
    let base_path_str = base_path.display().to_string();
    let output = format_projects(dirs, &base_path_str);

    bot.send_message(msg.chat.id, output)
        .await
        .map_err(|e| crate::types::error::OutpostError::telegram_error(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_list_project_dirs_with_subdirs() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("project-a")).unwrap();
        std::fs::create_dir(dir.path().join("project-b")).unwrap();
        std::fs::create_dir(dir.path().join("project-c")).unwrap();

        let dirs = list_project_dirs(&dir.path().to_path_buf());
        assert_eq!(dirs, vec!["project-a", "project-b", "project-c"]);
    }

    #[test]
    fn test_list_project_dirs_empty() {
        let dir = TempDir::new().unwrap();
        let dirs = list_project_dirs(&dir.path().to_path_buf());
        assert!(dirs.is_empty());
    }

    #[test]
    fn test_list_project_dirs_only_dirs_not_files() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("real-project")).unwrap();
        std::fs::write(dir.path().join("not-a-dir.txt"), "hello").unwrap();

        let dirs = list_project_dirs(&dir.path().to_path_buf());
        assert_eq!(dirs, vec!["real-project"]);
    }

    #[test]
    fn test_list_project_dirs_sorted() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("zebra")).unwrap();
        std::fs::create_dir(dir.path().join("alpha")).unwrap();
        std::fs::create_dir(dir.path().join("middle")).unwrap();

        let dirs = list_project_dirs(&dir.path().to_path_buf());
        assert_eq!(dirs, vec!["alpha", "middle", "zebra"]);
    }

    #[test]
    fn test_list_project_dirs_nonexistent_path() {
        let dirs = list_project_dirs(&PathBuf::from("/nonexistent/path/12345"));
        assert!(dirs.is_empty());
    }

    #[test]
    fn test_format_projects_empty() {
        let output = format_projects(vec![], "/home/user/projects");
        assert_eq!(output, "No projects found in `/home/user/projects`");
    }

    #[test]
    fn test_format_projects_single() {
        let output = format_projects(vec!["my-project".to_string()], "/home/user/projects");
        assert!(output.contains("📁 Available Projects"));
        assert!(output.contains("• `my-project`"));
    }

    #[test]
    fn test_format_projects_multiple() {
        let dirs = vec![
            "project-a".to_string(),
            "project-b".to_string(),
            "project-c".to_string(),
        ];
        let output = format_projects(dirs, "/home/user/projects");
        assert!(output.contains("📁 Available Projects"));
        assert!(output.contains("• `project-a`"));
        assert!(output.contains("• `project-b`"));
        assert!(output.contains("• `project-c`"));
    }

    #[test]
    fn test_format_projects_preserves_order() {
        let dirs = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];
        let output = format_projects(dirs, "/base");
        let alpha_pos = output.find("alpha").unwrap();
        let beta_pos = output.find("beta").unwrap();
        let gamma_pos = output.find("gamma").unwrap();
        assert!(alpha_pos < beta_pos && beta_pos < gamma_pos);
    }
}
