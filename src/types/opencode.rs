use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub title: Option<String>,
    pub created: i64,
    pub updated: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum MessagePart {
    Text { text: String },
    File(FilePart),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilePart {
    #[serde(rename = "type")]
    pub part_type: String,
    pub mime: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

impl FilePart {
    pub fn new(mime: &str, file_path: &Path) -> Self {
        Self {
            part_type: "file".to_string(),
            mime: mime.to_string(),
            url: format!("file://{}", file_path.display()),
            filename: file_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(String::from),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: Vec<MessagePart>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    pub message: Message,
    pub stream: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_info_deserialization() {
        let json = r#"{
            "id": "session-123",
            "title": "My Session",
            "created": 1640000000,
            "updated": 1640000100
        }"#;

        let session: SessionInfo = serde_json::from_str(json).unwrap();
        assert_eq!(session.id, "session-123");
        assert_eq!(session.title, Some("My Session".to_string()));
        assert_eq!(session.created, 1640000000);
        assert_eq!(session.updated, 1640000100);
    }

    #[test]
    fn test_session_info_without_title() {
        let json = r#"{
            "id": "session-456",
            "title": null,
            "created": 1650000000,
            "updated": 1650000200
        }"#;

        let session: SessionInfo = serde_json::from_str(json).unwrap();
        assert_eq!(session.id, "session-456");
        assert_eq!(session.title, None);
    }

    #[test]
    fn test_message_part_text() {
        let json = r#"{
            "type": "text",
            "text": "Hello, world!"
        }"#;

        let part: MessagePart = serde_json::from_str(json).unwrap();
        match part {
            MessagePart::Text { text } => assert_eq!(text, "Hello, world!"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_file_part_serialization() {
        let file_part = FilePart::new("image/jpeg", Path::new("/tmp/test.jpg"));
        let json = serde_json::to_string(&file_part).unwrap();
        assert!(json.contains(r#""type":"file""#));
        assert!(json.contains(r#""mime":"image/jpeg""#));
        assert!(json.contains(r#""url":"file:///tmp/test.jpg""#));
        assert!(json.contains(r#""filename":"test.jpg""#));
    }

    #[test]
    fn test_message_deserialization() {
        let json = r#"{
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": "Hello!"
                }
            ]
        }"#;

        let message: Message = serde_json::from_str(json).unwrap();
        assert_eq!(message.role, "user");
        assert_eq!(message.content.len(), 1);
    }

    #[test]
    fn test_create_message_request() {
        let json = r#"{
            "message": {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "Test message"
                    }
                ]
            },
            "stream": true
        }"#;

        let request: CreateMessageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.stream, Some(true));
        assert_eq!(request.message.role, "user");
    }

    #[test]
    fn test_session_info_serialization_roundtrip() {
        let session = SessionInfo {
            id: "test-session".to_string(),
            title: Some("Test".to_string()),
            created: 1640000000,
            updated: 1640000100,
        };

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: SessionInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, session.id);
        assert_eq!(deserialized.title, session.title);
        assert_eq!(deserialized.created, session.created);
        assert_eq!(deserialized.updated, session.updated);
    }
}
