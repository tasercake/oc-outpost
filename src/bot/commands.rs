use teloxide::utils::command::BotCommands;

/// Bot commands for oc-outpost
#[derive(BotCommands, Clone, Debug, PartialEq)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    /// Create new project and session
    #[command(description = "create new project and session - Usage: /new <project_name>")]
    New(String),

    /// List all sessions
    #[command(description = "list all sessions")]
    Sessions,

    /// List available projects
    #[command(description = "list available projects")]
    Projects,

    /// Close topic and clean up
    #[command(description = "close topic and clean up")]
    Close,

    /// Show session info
    #[command(description = "show current session info")]
    Session,

    /// Show orchestrator status
    #[command(description = "show orchestrator status")]
    Status,

    /// Show help
    #[command(description = "display this help text")]
    Help,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_new_command() {
        let cmd = Command::parse("/new my-project", "bot").unwrap();
        assert_eq!(cmd, Command::New("my-project".to_string()));
    }

    #[test]
    fn test_parse_sessions_command() {
        let cmd = Command::parse("/sessions", "bot").unwrap();
        assert_eq!(cmd, Command::Sessions);
    }

    #[test]
    fn test_parse_close_command() {
        let cmd = Command::parse("/close", "bot").unwrap();
        assert_eq!(cmd, Command::Close);
    }

    #[test]
    fn test_parse_projects_command() {
        let cmd = Command::parse("/projects", "bot").unwrap();
        assert_eq!(cmd, Command::Projects);
    }

    #[test]
    fn test_parse_session_command() {
        let cmd = Command::parse("/session", "bot").unwrap();
        assert_eq!(cmd, Command::Session);
    }

    #[test]
    fn test_parse_status_command() {
        let cmd = Command::parse("/status", "bot").unwrap();
        assert_eq!(cmd, Command::Status);
    }

    #[test]
    fn test_parse_help_command() {
        let cmd = Command::parse("/help", "bot").unwrap();
        assert_eq!(cmd, Command::Help);
    }

    #[test]
    fn test_command_descriptions() {
        let descriptions = Command::descriptions();
        assert!(descriptions.to_string().contains("create new project"));
        assert!(descriptions.to_string().contains("list all sessions"));
        assert!(descriptions.to_string().contains("list available projects"));
        assert!(descriptions
            .to_string()
            .contains("close topic and clean up"));
    }

    #[test]
    fn test_invalid_command() {
        let result = Command::parse("/invalid", "bot");
        assert!(result.is_err());
    }
}
