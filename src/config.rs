use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::time::Duration;

/// Configuration for oc-outpost loaded from environment variables
#[derive(Debug, Clone)]
pub struct Config {
    // Telegram (4 fields)
    pub telegram_bot_token: String,
    pub telegram_chat_id: i64,
    pub telegram_allowed_users: Vec<i64>,
    pub handle_general_topic: bool,

    // OpenCode (7 fields)
    pub opencode_path: PathBuf,
    pub opencode_max_instances: usize,
    pub opencode_idle_timeout: Duration,
    pub opencode_port_start: u16,
    pub opencode_port_pool_size: u16,
    pub opencode_health_check_interval: Duration,
    pub opencode_startup_timeout: Duration,

    // Storage (2 fields)
    pub orchestrator_db_path: PathBuf,
    pub topic_db_path: PathBuf,

    // Project (2 fields)
    pub project_base_path: PathBuf,
    pub auto_create_project_dirs: bool,

    // API (2 fields)
    pub api_port: u16,
    pub api_key: Option<String>,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let telegram_bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| anyhow!("TELEGRAM_BOT_TOKEN is required but not set"))?;

        let telegram_chat_id = std::env::var("TELEGRAM_CHAT_ID")
            .map_err(|_| anyhow!("TELEGRAM_CHAT_ID is required but not set"))?
            .parse::<i64>()
            .map_err(|_| anyhow!("TELEGRAM_CHAT_ID must be a valid integer"))?;

        let telegram_allowed_users = std::env::var("TELEGRAM_ALLOWED_USERS")
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                s.trim()
                    .parse::<i64>()
                    .map_err(|_| anyhow!("TELEGRAM_ALLOWED_USERS contains invalid integer"))
            })
            .collect::<Result<Vec<_>>>()?;

        let handle_general_topic = std::env::var("HANDLE_GENERAL_TOPIC")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .map_err(|_| anyhow!("HANDLE_GENERAL_TOPIC must be 'true' or 'false'"))?;

        let opencode_path = PathBuf::from(
            std::env::var("OPENCODE_PATH").unwrap_or_else(|_| "opencode".to_string()),
        );

        let opencode_max_instances = std::env::var("OPENCODE_MAX_INSTANCES")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<usize>()
            .map_err(|_| anyhow!("OPENCODE_MAX_INSTANCES must be a valid integer"))?;

        let opencode_idle_timeout = Duration::from_millis(
            std::env::var("OPENCODE_IDLE_TIMEOUT_MS")
                .unwrap_or_else(|_| "1800000".to_string())
                .parse::<u64>()
                .map_err(|_| anyhow!("OPENCODE_IDLE_TIMEOUT_MS must be a valid integer"))?,
        );

        let opencode_port_start = std::env::var("OPENCODE_PORT_START")
            .unwrap_or_else(|_| "4100".to_string())
            .parse::<u16>()
            .map_err(|_| anyhow!("OPENCODE_PORT_START must be a valid port number"))?;

        let opencode_port_pool_size = std::env::var("OPENCODE_PORT_POOL_SIZE")
            .unwrap_or_else(|_| "100".to_string())
            .parse::<u16>()
            .map_err(|_| anyhow!("OPENCODE_PORT_POOL_SIZE must be a valid integer"))?;

        let opencode_health_check_interval = Duration::from_millis(
            std::env::var("OPENCODE_HEALTH_CHECK_INTERVAL_MS")
                .unwrap_or_else(|_| "30000".to_string())
                .parse::<u64>()
                .map_err(|_| {
                    anyhow!("OPENCODE_HEALTH_CHECK_INTERVAL_MS must be a valid integer")
                })?,
        );

        let opencode_startup_timeout = Duration::from_millis(
            std::env::var("OPENCODE_STARTUP_TIMEOUT_MS")
                .unwrap_or_else(|_| "60000".to_string())
                .parse::<u64>()
                .map_err(|_| anyhow!("OPENCODE_STARTUP_TIMEOUT_MS must be a valid integer"))?,
        );

        let orchestrator_db_path = PathBuf::from(
            std::env::var("ORCHESTRATOR_DB_PATH")
                .unwrap_or_else(|_| "./data/orchestrator.db".to_string()),
        );

        let topic_db_path = PathBuf::from(
            std::env::var("TOPIC_DB_PATH").unwrap_or_else(|_| "./data/topics.db".to_string()),
        );

        let project_base_path = std::env::var("PROJECT_BASE_PATH")
            .map_err(|_| anyhow!("PROJECT_BASE_PATH is required but not set"))?;
        let project_base_path = PathBuf::from(shellexpand::tilde(&project_base_path).into_owned());

        let auto_create_project_dirs = std::env::var("AUTO_CREATE_PROJECT_DIRS")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .map_err(|_| anyhow!("AUTO_CREATE_PROJECT_DIRS must be 'true' or 'false'"))?;

        let api_port = std::env::var("API_PORT")
            .unwrap_or_else(|_| "4200".to_string())
            .parse::<u16>()
            .map_err(|_| anyhow!("API_PORT must be a valid port number"))?;

        let api_key = std::env::var("API_KEY").ok();

        Ok(Config {
            telegram_bot_token,
            telegram_chat_id,
            telegram_allowed_users,
            handle_general_topic,
            opencode_path,
            opencode_max_instances,
            opencode_idle_timeout,
            opencode_port_start,
            opencode_port_pool_size,
            opencode_health_check_interval,
            opencode_startup_timeout,
            orchestrator_db_path,
            topic_db_path,
            project_base_path,
            auto_create_project_dirs,
            api_port,
            api_key,
        })
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Config {{\n  telegram_bot_token: ***MASKED***,\n  telegram_chat_id: {},\n  telegram_allowed_users: {:?},\n  handle_general_topic: {},\n  opencode_path: {:?},\n  opencode_max_instances: {},\n  opencode_idle_timeout: {:?},\n  opencode_port_start: {},\n  opencode_port_pool_size: {},\n  opencode_health_check_interval: {:?},\n  opencode_startup_timeout: {:?},\n  orchestrator_db_path: {:?},\n  topic_db_path: {:?},\n  project_base_path: {:?},\n  auto_create_project_dirs: {},\n  api_port: {},\n  api_key: {},\n}}",
            self.telegram_chat_id,
            self.telegram_allowed_users,
            self.handle_general_topic,
            self.opencode_path,
            self.opencode_max_instances,
            self.opencode_idle_timeout,
            self.opencode_port_start,
            self.opencode_port_pool_size,
            self.opencode_health_check_interval,
            self.opencode_startup_timeout,
            self.orchestrator_db_path,
            self.topic_db_path,
            self.project_base_path,
            self.auto_create_project_dirs,
            self.api_port,
            if self.api_key.is_some() { "***MASKED***" } else { "None" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_missing_telegram_bot_token() {
        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        std::env::set_var("TELEGRAM_CHAT_ID", "-1001234567890");
        std::env::set_var("PROJECT_BASE_PATH", "~/oc-bot");

        let result = Config::from_env();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("TELEGRAM_BOT_TOKEN is required"));
    }

    #[test]
    #[serial]
    fn test_missing_telegram_chat_id() {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::remove_var("TELEGRAM_CHAT_ID");
        std::env::set_var("PROJECT_BASE_PATH", "~/oc-bot");

        let result = Config::from_env();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("TELEGRAM_CHAT_ID is required"));
    }

    #[test]
    #[serial]
    fn test_missing_project_base_path() {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "-1001234567890");
        std::env::remove_var("PROJECT_BASE_PATH");

        let result = Config::from_env();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("PROJECT_BASE_PATH is required"));
    }

    #[test]
    #[serial]
    fn test_defaults_applied_correctly() {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "-1001234567890");
        std::env::set_var("PROJECT_BASE_PATH", "~/oc-bot");

        std::env::remove_var("OPENCODE_PATH");
        std::env::remove_var("OPENCODE_MAX_INSTANCES");
        std::env::remove_var("OPENCODE_IDLE_TIMEOUT_MS");
        std::env::remove_var("OPENCODE_PORT_START");
        std::env::remove_var("OPENCODE_PORT_POOL_SIZE");
        std::env::remove_var("OPENCODE_HEALTH_CHECK_INTERVAL_MS");
        std::env::remove_var("OPENCODE_STARTUP_TIMEOUT_MS");
        std::env::remove_var("ORCHESTRATOR_DB_PATH");
        std::env::remove_var("TOPIC_DB_PATH");
        std::env::remove_var("AUTO_CREATE_PROJECT_DIRS");
        std::env::remove_var("API_PORT");
        std::env::remove_var("TELEGRAM_ALLOWED_USERS");
        std::env::remove_var("HANDLE_GENERAL_TOPIC");
        std::env::remove_var("API_KEY");

        let config = Config::from_env().expect("Config should load with defaults");

        assert_eq!(config.opencode_path, PathBuf::from("opencode"));
        assert_eq!(config.opencode_max_instances, 10);
        assert_eq!(config.opencode_idle_timeout, Duration::from_millis(1800000));
        assert_eq!(config.opencode_port_start, 4100);
        assert_eq!(config.opencode_port_pool_size, 100);
        assert_eq!(
            config.opencode_health_check_interval,
            Duration::from_millis(30000)
        );
        assert_eq!(
            config.opencode_startup_timeout,
            Duration::from_millis(60000)
        );
        assert_eq!(
            config.orchestrator_db_path,
            PathBuf::from("./data/orchestrator.db")
        );
        assert_eq!(config.topic_db_path, PathBuf::from("./data/topics.db"));
        assert!(config.auto_create_project_dirs);
        assert_eq!(config.api_port, 4200);
        assert!(config.handle_general_topic);
        assert!(config.telegram_allowed_users.is_empty());
        assert!(config.api_key.is_none());
    }

    #[test]
    #[serial]
    fn test_duration_parsing() {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "-1001234567890");
        std::env::set_var("PROJECT_BASE_PATH", "~/oc-bot");
        std::env::set_var("OPENCODE_IDLE_TIMEOUT_MS", "5000");
        std::env::set_var("OPENCODE_HEALTH_CHECK_INTERVAL_MS", "15000");
        std::env::set_var("OPENCODE_STARTUP_TIMEOUT_MS", "30000");

        let config = Config::from_env().expect("Config should parse durations");

        assert_eq!(config.opencode_idle_timeout, Duration::from_millis(5000));
        assert_eq!(
            config.opencode_health_check_interval,
            Duration::from_millis(15000)
        );
        assert_eq!(
            config.opencode_startup_timeout,
            Duration::from_millis(30000)
        );
    }

    #[test]
    #[serial]
    fn test_path_expansion() {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "-1001234567890");
        std::env::set_var("PROJECT_BASE_PATH", "~/oc-bot");

        let config = Config::from_env().expect("Config should expand paths");

        assert!(!config.project_base_path.to_string_lossy().contains("~"));
        assert!(!config.project_base_path.to_string_lossy().is_empty());
    }

    #[test]
    #[serial]
    fn test_telegram_allowed_users_parsing() {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "-1001234567890");
        std::env::set_var("PROJECT_BASE_PATH", "~/oc-bot");
        std::env::set_var("TELEGRAM_ALLOWED_USERS", "123,456,789");

        let config = Config::from_env().expect("Config should parse allowed users");

        assert_eq!(config.telegram_allowed_users, vec![123, 456, 789]);
    }

    #[test]
    #[serial]
    fn test_telegram_allowed_users_empty() {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "-1001234567890");
        std::env::set_var("PROJECT_BASE_PATH", "~/oc-bot");
        std::env::set_var("TELEGRAM_ALLOWED_USERS", "");

        let config = Config::from_env().expect("Config should handle empty allowed users");

        assert!(config.telegram_allowed_users.is_empty());
    }

    #[test]
    #[serial]
    fn test_masked_display() {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "secret-token-12345");
        std::env::set_var("TELEGRAM_CHAT_ID", "-1001234567890");
        std::env::set_var("PROJECT_BASE_PATH", "~/oc-bot");
        std::env::set_var("API_KEY", "secret-api-key");

        let config = Config::from_env().expect("Config should load");
        let display = config.to_string();

        assert!(display.contains("***MASKED***"));
        assert!(!display.contains("secret-token-12345"));
        assert!(!display.contains("secret-api-key"));
    }

    #[test]
    #[serial]
    fn test_invalid_telegram_chat_id() {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "not-a-number");
        std::env::set_var("PROJECT_BASE_PATH", "~/oc-bot");

        let result = Config::from_env();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("TELEGRAM_CHAT_ID must be a valid integer"));
    }

    #[test]
    #[serial]
    fn test_invalid_opencode_max_instances() {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "-1001234567890");
        std::env::set_var("PROJECT_BASE_PATH", "~/oc-bot");
        std::env::set_var("OPENCODE_MAX_INSTANCES", "not-a-number");

        let result = Config::from_env();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("OPENCODE_MAX_INSTANCES must be a valid integer"));
    }

    #[test]
    #[serial]
    fn test_invalid_api_port() {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "-1001234567890");
        std::env::set_var("PROJECT_BASE_PATH", "~/oc-bot");
        std::env::set_var("API_PORT", "not-a-port");

        let result = Config::from_env();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("API_PORT must be a valid port number"));
    }

    #[test]
    #[serial]
    fn test_invalid_boolean_field() {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test-token");
        std::env::set_var("TELEGRAM_CHAT_ID", "-1001234567890");
        std::env::set_var("PROJECT_BASE_PATH", "~/oc-bot");
        std::env::set_var("HANDLE_GENERAL_TOPIC", "maybe");

        let result = Config::from_env();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("HANDLE_GENERAL_TOPIC must be 'true' or 'false'"));
    }

    #[test]
    #[serial]
    fn test_full_config_load() {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "123456789:ABCdefGHIjklMNOpqrsTUVwxyz");
        std::env::set_var("TELEGRAM_CHAT_ID", "-1001234567890");
        std::env::set_var("TELEGRAM_ALLOWED_USERS", "111,222,333");
        std::env::set_var("HANDLE_GENERAL_TOPIC", "true");
        std::env::set_var("OPENCODE_PATH", "/usr/local/bin/opencode");
        std::env::set_var("OPENCODE_MAX_INSTANCES", "20");
        std::env::set_var("OPENCODE_IDLE_TIMEOUT_MS", "3600000");
        std::env::set_var("OPENCODE_PORT_START", "5000");
        std::env::set_var("OPENCODE_PORT_POOL_SIZE", "50");
        std::env::set_var("OPENCODE_HEALTH_CHECK_INTERVAL_MS", "45000");
        std::env::set_var("OPENCODE_STARTUP_TIMEOUT_MS", "90000");
        std::env::set_var("ORCHESTRATOR_DB_PATH", "./custom/orchestrator.db");
        std::env::set_var("TOPIC_DB_PATH", "./custom/topics.db");
        std::env::set_var("PROJECT_BASE_PATH", "~/projects");
        std::env::set_var("AUTO_CREATE_PROJECT_DIRS", "false");
        std::env::set_var("API_PORT", "8080");
        std::env::set_var("API_KEY", "my-secret-key");

        let config = Config::from_env().expect("Config should load all fields");

        assert_eq!(
            config.telegram_bot_token,
            "123456789:ABCdefGHIjklMNOpqrsTUVwxyz"
        );
        assert_eq!(config.telegram_chat_id, -1001234567890);
        assert_eq!(config.telegram_allowed_users, vec![111, 222, 333]);
        assert!(config.handle_general_topic);
        assert_eq!(
            config.opencode_path,
            PathBuf::from("/usr/local/bin/opencode")
        );
        assert_eq!(config.opencode_max_instances, 20);
        assert_eq!(config.opencode_idle_timeout, Duration::from_millis(3600000));
        assert_eq!(config.opencode_port_start, 5000);
        assert_eq!(config.opencode_port_pool_size, 50);
        assert_eq!(
            config.opencode_health_check_interval,
            Duration::from_millis(45000)
        );
        assert_eq!(
            config.opencode_startup_timeout,
            Duration::from_millis(90000)
        );
        assert_eq!(
            config.orchestrator_db_path,
            PathBuf::from("./custom/orchestrator.db")
        );
        assert_eq!(config.topic_db_path, PathBuf::from("./custom/topics.db"));
        assert!(!config.auto_create_project_dirs);
        assert_eq!(config.api_port, 8080);
        assert_eq!(config.api_key, Some("my-secret-key".to_string()));
    }
}
