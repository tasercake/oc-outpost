use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopicMapping {
    pub topic_id: i32,
    pub chat_id: i64,
    pub project_path: String,
    pub session_id: Option<String>,
    pub instance_id: Option<String>,
    pub topic_name_updated: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_mapping_deserialization() {
        let json = r#"{
            "topic_id": 123,
            "chat_id": -1001234567890,
            "project_path": "/path/to/project",
            "session_id": "session-123",
            "instance_id": "instance-456",
            "topic_name_updated": false,
            "created_at": 1640000000,
            "updated_at": 1640000100
        }"#;

        let mapping: TopicMapping = serde_json::from_str(json).unwrap();
        assert_eq!(mapping.topic_id, 123);
        assert_eq!(mapping.chat_id, -1001234567890);
        assert_eq!(mapping.project_path, "/path/to/project");
        assert_eq!(mapping.session_id, Some("session-123".to_string()));
        assert_eq!(mapping.instance_id, Some("instance-456".to_string()));
        assert!(!mapping.topic_name_updated);
        assert_eq!(mapping.created_at, 1640000000);
        assert_eq!(mapping.updated_at, 1640000100);
    }

    #[test]
    fn test_topic_mapping_with_null_fields() {
        let json = r#"{
            "topic_id": 123,
            "chat_id": -1001234567890,
            "project_path": "/path/to/project",
            "session_id": null,
            "instance_id": null,
            "topic_name_updated": true,
            "created_at": 1640000000,
            "updated_at": 1640000100
        }"#;

        let mapping: TopicMapping = serde_json::from_str(json).unwrap();
        assert_eq!(mapping.session_id, None);
        assert_eq!(mapping.instance_id, None);
        assert!(mapping.topic_name_updated);
    }

    #[test]
    fn test_topic_mapping_serialization_roundtrip() {
        let mapping = TopicMapping {
            topic_id: 456,
            chat_id: -1009876543210,
            project_path: "/another/path".to_string(),
            session_id: Some("sess-789".to_string()),
            instance_id: None,
            topic_name_updated: false,
            created_at: 1650000000,
            updated_at: 1650000200,
        };

        let json = serde_json::to_string(&mapping).unwrap();
        let deserialized: TopicMapping = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.topic_id, mapping.topic_id);
        assert_eq!(deserialized.chat_id, mapping.chat_id);
        assert_eq!(deserialized.project_path, mapping.project_path);
        assert_eq!(deserialized.session_id, mapping.session_id);
        assert_eq!(deserialized.instance_id, mapping.instance_id);
        assert_eq!(deserialized.topic_name_updated, mapping.topic_name_updated);
        assert_eq!(deserialized.created_at, mapping.created_at);
        assert_eq!(deserialized.updated_at, mapping.updated_at);
    }

    #[test]
    fn test_topic_mapping_clone() {
        let mapping = TopicMapping {
            topic_id: 789,
            chat_id: -1001111111111,
            project_path: "/test/path".to_string(),
            session_id: Some("test-session".to_string()),
            instance_id: Some("test-instance".to_string()),
            topic_name_updated: true,
            created_at: 1660000000,
            updated_at: 1660000300,
        };

        let cloned = mapping.clone();
        assert_eq!(cloned.topic_id, mapping.topic_id);
        assert_eq!(cloned.chat_id, mapping.chat_id);
        assert_eq!(cloned.project_path, mapping.project_path);
    }
}
