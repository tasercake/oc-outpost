use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstanceState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub id: String,
    pub project_path: String,
    pub port: u16,
    pub auto_start: bool,
    #[serde(default = "default_opencode_path")]
    pub opencode_path: String,
}

fn default_opencode_path() -> String {
    "opencode".to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: String,
    pub state: InstanceState,
    pub project_path: String,
    pub port: u16,
    pub pid: Option<u32>,
    pub container_id: Option<String>,
    pub started_at: Option<i64>,
    pub stopped_at: Option<i64>,
    pub topic_id: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_state_serialization() {
        let state = InstanceState::Running;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, r#""running""#);

        let deserialized: InstanceState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, InstanceState::Running);
    }

    #[test]
    fn test_instance_state_all_variants() {
        let states = vec![
            (InstanceState::Starting, r#""starting""#),
            (InstanceState::Running, r#""running""#),
            (InstanceState::Stopping, r#""stopping""#),
            (InstanceState::Stopped, r#""stopped""#),
            (InstanceState::Error, r#""error""#),
        ];

        for (state, expected_json) in states {
            let json = serde_json::to_string(&state).unwrap();
            assert_eq!(json, expected_json);

            let deserialized: InstanceState = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, state);
        }
    }

    #[test]
    fn test_instance_config_deserialization() {
        let json = r#"{
            "id": "test-instance",
            "project_path": "/path/to/project",
            "port": 3000,
            "auto_start": true
        }"#;

        let config: InstanceConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.id, "test-instance");
        assert_eq!(config.project_path, "/path/to/project");
        assert_eq!(config.port, 3000);
        assert!(config.auto_start);
    }

    #[test]
    fn test_instance_config_serialization_roundtrip() {
        let config = InstanceConfig {
            id: "test-instance".to_string(),
            project_path: "/path/to/project".to_string(),
            port: 8080,
            auto_start: false,
            opencode_path: "opencode".to_string(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: InstanceConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, config.id);
        assert_eq!(deserialized.project_path, config.project_path);
        assert_eq!(deserialized.port, config.port);
        assert_eq!(deserialized.auto_start, config.auto_start);
    }

    #[test]
    fn test_instance_info_deserialization() {
        let json = r#"{
            "id": "test-instance",
            "state": "running",
            "project_path": "/path/to/project",
            "port": 3000,
            "pid": 12345,
            "started_at": 1640000000,
            "stopped_at": null,
            "topic_id": 0
        }"#;

        let info: InstanceInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.id, "test-instance");
        assert_eq!(info.state, InstanceState::Running);
        assert_eq!(info.project_path, "/path/to/project");
        assert_eq!(info.port, 3000);
        assert_eq!(info.pid, Some(12345));
        assert_eq!(info.started_at, Some(1640000000));
        assert_eq!(info.stopped_at, None);
    }

    #[test]
    fn test_instance_info_with_null_fields() {
        let json = r#"{
            "id": "test-instance",
            "state": "stopped",
            "project_path": "/path/to/project",
            "port": 3000,
            "pid": null,
            "started_at": null,
            "stopped_at": 1640000000,
            "topic_id": 0
        }"#;

        let info: InstanceInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.state, InstanceState::Stopped);
        assert_eq!(info.pid, None);
        assert_eq!(info.started_at, None);
        assert_eq!(info.stopped_at, Some(1640000000));
    }

    #[test]
    fn test_instance_info_serialization_roundtrip() {
        let info = InstanceInfo {
            id: "test-instance".to_string(),
            state: InstanceState::Running,
            project_path: "/path/to/project".to_string(),
            port: 3000,
            pid: Some(12345),
            container_id: None,
            started_at: Some(1640000000),
            stopped_at: None,
            topic_id: 0,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: InstanceInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, info.id);
        assert_eq!(deserialized.state, info.state);
        assert_eq!(deserialized.project_path, info.project_path);
        assert_eq!(deserialized.port, info.port);
        assert_eq!(deserialized.pid, info.pid);
        assert_eq!(deserialized.started_at, info.started_at);
        assert_eq!(deserialized.stopped_at, info.stopped_at);
    }
}
