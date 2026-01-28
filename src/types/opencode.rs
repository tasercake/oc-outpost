use serde::{Deserialize, Serialize};

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
    Image { source: ImageSource },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ImageSource {
    Url { url: String },
    Base64 { media_type: String, data: String },
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum SseEvent {
    MessageStart {
        message: MessageMetadata,
    },
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    ContentBlockDelta {
        index: usize,
        delta: ContentDelta,
    },
    ContentBlockStop {
        index: usize,
    },
    MessageDelta {
        delta: MessageDeltaData,
    },
    MessageStop,
    Error {
        error: ErrorData,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub id: String,
    pub role: String,
    pub model: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum ContentDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageDeltaData {
    pub stop_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ErrorData {
    pub message: String,
    pub code: Option<String>,
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
    fn test_message_part_image_url() {
        let json = r#"{
            "type": "image",
            "source": {
                "type": "url",
                "url": "https://example.com/image.png"
            }
        }"#;

        let part: MessagePart = serde_json::from_str(json).unwrap();
        match part {
            MessagePart::Image { source } => match source {
                ImageSource::Url { url } => assert_eq!(url, "https://example.com/image.png"),
                _ => panic!("Expected Url variant"),
            },
            _ => panic!("Expected Image variant"),
        }
    }

    #[test]
    fn test_message_part_image_base64() {
        let json = r#"{
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": "image/png",
                "data": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
            }
        }"#;

        let part: MessagePart = serde_json::from_str(json).unwrap();
        match part {
            MessagePart::Image { source } => match source {
                ImageSource::Base64 { media_type, data } => {
                    assert_eq!(media_type, "image/png");
                    assert!(!data.is_empty());
                }
                _ => panic!("Expected Base64 variant"),
            },
            _ => panic!("Expected Image variant"),
        }
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
    fn test_sse_event_message_start() {
        let json = r#"{
            "type": "message_start",
            "message": {
                "id": "msg-123",
                "role": "assistant",
                "model": "claude-3-opus"
            }
        }"#;

        let event: SseEvent = serde_json::from_str(json).unwrap();
        match event {
            SseEvent::MessageStart { message } => {
                assert_eq!(message.id, "msg-123");
                assert_eq!(message.role, "assistant");
                assert_eq!(message.model, Some("claude-3-opus".to_string()));
            }
            _ => panic!("Expected MessageStart variant"),
        }
    }

    #[test]
    fn test_sse_event_content_block_delta() {
        let json = r#"{
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "text_delta",
                "text": "Hello"
            }
        }"#;

        let event: SseEvent = serde_json::from_str(json).unwrap();
        match event {
            SseEvent::ContentBlockDelta { index, delta } => {
                assert_eq!(index, 0);
                match delta {
                    ContentDelta::TextDelta { text } => assert_eq!(text, "Hello"),
                    _ => panic!("Expected TextDelta variant"),
                }
            }
            _ => panic!("Expected ContentBlockDelta variant"),
        }
    }

    #[test]
    fn test_sse_event_message_stop() {
        let json = r#"{
            "type": "message_stop"
        }"#;

        let event: SseEvent = serde_json::from_str(json).unwrap();
        match event {
            SseEvent::MessageStop => {}
            _ => panic!("Expected MessageStop variant"),
        }
    }

    #[test]
    fn test_sse_event_error() {
        let json = r#"{
            "type": "error",
            "error": {
                "message": "Something went wrong",
                "code": "internal_error"
            }
        }"#;

        let event: SseEvent = serde_json::from_str(json).unwrap();
        match event {
            SseEvent::Error { error } => {
                assert_eq!(error.message, "Something went wrong");
                assert_eq!(error.code, Some("internal_error".to_string()));
            }
            _ => panic!("Expected Error variant"),
        }
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
