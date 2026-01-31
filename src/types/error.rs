use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum OutpostError {
    #[error("Instance not found: {id}")]
    #[allow(dead_code)]
    // Used by future: instance lookup error handling
    InstanceNotFound { id: String },

    #[error("Instance already exists: {id}")]
    #[allow(dead_code)]
    // Used by future: instance creation validation
    InstanceAlreadyExists { id: String },

    #[error("Instance failed to start: {id}, reason: {reason}")]
    #[allow(dead_code)]
    // Used by future: instance startup error handling
    InstanceStartFailed { id: String, reason: String },

    #[error("Instance failed to stop: {id}, reason: {reason}")]
    #[allow(dead_code)]
    // Used by future: instance shutdown error handling
    InstanceStopFailed { id: String, reason: String },

    #[error("Topic mapping not found for topic_id: {topic_id}")]
    #[allow(dead_code)]
    // Used by future: topic mapping lookup error handling
    TopicMappingNotFound { topic_id: i32 },

    #[error("Topic mapping already exists for topic_id: {topic_id}")]
    #[allow(dead_code)]
    // Used by future: topic mapping creation validation
    TopicMappingAlreadyExists { topic_id: i32 },

    #[error("OpenCode API error: {message}")]
    OpenCodeApiError { message: String },

    #[error("OpenCode connection error: {url}, reason: {reason}")]
    #[allow(dead_code)]
    // Used by future: OpenCode connection error handling
    OpenCodeConnectionError { url: String, reason: String },

    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("Database error: {message}")]
    DatabaseError { message: String },

    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    #[error("Port allocation error: no available ports in range {start}-{end}")]
    #[allow(dead_code)]
    // Used by future: port allocation error handling
    PortAllocationError { start: u16, end: u16 },

    #[error("Invalid state transition: from {from} to {to}")]
    #[allow(dead_code)]
    // Used by future: state validation error handling
    InvalidStateTransition { from: String, to: String },

    #[error("Maximum instances limit reached: {limit}")]
    #[allow(dead_code)]
    // Used by future: instance limit enforcement
    MaxInstancesReached { limit: usize },

    #[error("IO error: {message}")]
    IoError { message: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    #[error("Telegram API error: {message}")]
    TelegramError { message: String },
}

impl OutpostError {
    /// Returns true if this error was caused by user input/action rather than a
    /// system failure. Used to downgrade error logging from ERROR to WARN in
    /// top-level command dispatch, since user-triggered errors (wrong command
    /// context, invalid input, etc.) are expected operational noise.
    pub fn is_user_error(&self) -> bool {
        matches!(
            self,
            Self::TelegramError { .. }
                | Self::ConfigError { .. }
                | Self::TopicMappingNotFound { .. }
                | Self::TopicMappingAlreadyExists { .. }
                | Self::SessionNotFound { .. }
                | Self::MaxInstancesReached { .. }
        )
    }

    #[allow(dead_code)]
    // Used by future: instance lookup error handling
    pub fn instance_not_found(id: impl Into<String>) -> Self {
        Self::InstanceNotFound { id: id.into() }
    }

    #[allow(dead_code)]
    // Used by future: instance creation validation
    pub fn instance_already_exists(id: impl Into<String>) -> Self {
        Self::InstanceAlreadyExists { id: id.into() }
    }

    #[allow(dead_code)]
    // Used by future: instance startup error handling
    pub fn instance_start_failed(id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InstanceStartFailed {
            id: id.into(),
            reason: reason.into(),
        }
    }

    #[allow(dead_code)]
    // Used by future: instance shutdown error handling
    pub fn instance_stop_failed(id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InstanceStopFailed {
            id: id.into(),
            reason: reason.into(),
        }
    }

    #[allow(dead_code)]
    // Used by future: topic mapping lookup error handling
    pub fn topic_mapping_not_found(topic_id: i32) -> Self {
        Self::TopicMappingNotFound { topic_id }
    }

    #[allow(dead_code)]
    // Used by future: topic mapping creation validation
    pub fn topic_mapping_already_exists(topic_id: i32) -> Self {
        Self::TopicMappingAlreadyExists { topic_id }
    }

    pub fn opencode_api_error(message: impl Into<String>) -> Self {
        Self::OpenCodeApiError {
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    // Used by future: OpenCode connection error handling
    pub fn opencode_connection_error(url: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::OpenCodeConnectionError {
            url: url.into(),
            reason: reason.into(),
        }
    }

    pub fn session_not_found(session_id: impl Into<String>) -> Self {
        Self::SessionNotFound {
            session_id: session_id.into(),
        }
    }

    pub fn database_error(message: impl Into<String>) -> Self {
        Self::DatabaseError {
            message: message.into(),
        }
    }

    pub fn config_error(message: impl Into<String>) -> Self {
        Self::ConfigError {
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    // Used by future: port allocation error handling
    pub fn port_allocation_error(start: u16, end: u16) -> Self {
        Self::PortAllocationError { start, end }
    }

    #[allow(dead_code)]
    // Used by future: state validation error handling
    pub fn invalid_state_transition(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::InvalidStateTransition {
            from: from.into(),
            to: to.into(),
        }
    }

    #[allow(dead_code)]
    // Used by future: instance limit enforcement
    pub fn max_instances_reached(limit: usize) -> Self {
        Self::MaxInstancesReached { limit }
    }

    pub fn io_error(message: impl Into<String>) -> Self {
        Self::IoError {
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    // Used by future: serialization error handling
    pub fn serialization_error(message: impl Into<String>) -> Self {
        Self::SerializationError {
            message: message.into(),
        }
    }

    pub fn telegram_error(message: impl Into<String>) -> Self {
        Self::TelegramError {
            message: message.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, OutpostError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_not_found_error() {
        let err = OutpostError::instance_not_found("test-instance");
        assert_eq!(err.to_string(), "Instance not found: test-instance");
    }

    #[test]
    fn test_instance_already_exists_error() {
        let err = OutpostError::instance_already_exists("existing-instance");
        assert_eq!(
            err.to_string(),
            "Instance already exists: existing-instance"
        );
    }

    #[test]
    fn test_instance_start_failed_error() {
        let err = OutpostError::instance_start_failed("test-instance", "port already in use");
        assert_eq!(
            err.to_string(),
            "Instance failed to start: test-instance, reason: port already in use"
        );
    }

    #[test]
    fn test_instance_stop_failed_error() {
        let err = OutpostError::instance_stop_failed("test-instance", "process not responding");
        assert_eq!(
            err.to_string(),
            "Instance failed to stop: test-instance, reason: process not responding"
        );
    }

    #[test]
    fn test_topic_mapping_not_found_error() {
        let err = OutpostError::topic_mapping_not_found(123);
        assert_eq!(err.to_string(), "Topic mapping not found for topic_id: 123");
    }

    #[test]
    fn test_topic_mapping_already_exists_error() {
        let err = OutpostError::topic_mapping_already_exists(456);
        assert_eq!(
            err.to_string(),
            "Topic mapping already exists for topic_id: 456"
        );
    }

    #[test]
    fn test_opencode_api_error() {
        let err = OutpostError::opencode_api_error("Invalid request");
        assert_eq!(err.to_string(), "OpenCode API error: Invalid request");
    }

    #[test]
    fn test_opencode_connection_error() {
        let err =
            OutpostError::opencode_connection_error("http://localhost:3000", "connection refused");
        assert_eq!(
            err.to_string(),
            "OpenCode connection error: http://localhost:3000, reason: connection refused"
        );
    }

    #[test]
    fn test_session_not_found_error() {
        let err = OutpostError::session_not_found("session-123");
        assert_eq!(err.to_string(), "Session not found: session-123");
    }

    #[test]
    fn test_database_error() {
        let err = OutpostError::database_error("Failed to connect");
        assert_eq!(err.to_string(), "Database error: Failed to connect");
    }

    #[test]
    fn test_config_error() {
        let err = OutpostError::config_error("Missing required field");
        assert_eq!(
            err.to_string(),
            "Configuration error: Missing required field"
        );
    }

    #[test]
    fn test_port_allocation_error() {
        let err = OutpostError::port_allocation_error(3000, 3100);
        assert_eq!(
            err.to_string(),
            "Port allocation error: no available ports in range 3000-3100"
        );
    }

    #[test]
    fn test_invalid_state_transition_error() {
        let err = OutpostError::invalid_state_transition("running", "starting");
        assert_eq!(
            err.to_string(),
            "Invalid state transition: from running to starting"
        );
    }

    #[test]
    fn test_max_instances_reached_error() {
        let err = OutpostError::max_instances_reached(10);
        assert_eq!(err.to_string(), "Maximum instances limit reached: 10");
    }

    #[test]
    fn test_io_error() {
        let err = OutpostError::io_error("File not found");
        assert_eq!(err.to_string(), "IO error: File not found");
    }

    #[test]
    fn test_serialization_error() {
        let err = OutpostError::serialization_error("Invalid JSON");
        assert_eq!(err.to_string(), "Serialization error: Invalid JSON");
    }

    #[test]
    fn test_telegram_error() {
        let err = OutpostError::telegram_error("Bot token invalid");
        assert_eq!(err.to_string(), "Telegram API error: Bot token invalid");
    }

    #[test]
    fn test_is_user_error_classification() {
        assert!(OutpostError::telegram_error("test").is_user_error());
        assert!(OutpostError::config_error("test").is_user_error());
        assert!(OutpostError::topic_mapping_not_found(1).is_user_error());
        assert!(OutpostError::topic_mapping_already_exists(1).is_user_error());
        assert!(OutpostError::session_not_found("test").is_user_error());
        assert!(OutpostError::max_instances_reached(10).is_user_error());

        assert!(!OutpostError::database_error("test").is_user_error());
        assert!(!OutpostError::io_error("test").is_user_error());
        assert!(!OutpostError::opencode_api_error("test").is_user_error());
        assert!(!OutpostError::instance_not_found("test").is_user_error());
        assert!(!OutpostError::port_allocation_error(3000, 3100).is_user_error());
    }

    #[test]
    fn test_error_clone() {
        let err = OutpostError::instance_not_found("test");
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.ok(), Some(42));
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(OutpostError::instance_not_found("test"));
        assert!(result.is_err());
    }
}
