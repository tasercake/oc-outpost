use crate::types::opencode::{CreateMessageRequest, Message, MessagePart, SessionInfo};
use anyhow::{Context, Result};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// OpenCode REST API client
#[derive(Clone)]
pub struct OpenCodeClient {
    client: reqwest::Client,
    base_url: String,
}

/// Metadata for a message response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ResponseMetadata {
    pub id: String,
    pub role: String,
    pub model: Option<String>,
}

/// Response from sending a message
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)]
// Used by future: send_message() method for synchronous message sending
pub struct MessageResponse {
    pub message: Message,
    pub metadata: ResponseMetadata,
}

/// Request body for creating a session
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)]
struct CreateSessionRequest {
    project_path: String,
}

/// Request body for permission reply
#[derive(Clone, Debug, Serialize, Deserialize)]
struct PermissionReplyRequest {
    allow: bool,
}

/// Health check response
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)]
struct HealthResponse {
    status: String,
}

impl OpenCodeClient {
    /// Create a new OpenCode client
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Check if the OpenCode server is healthy
    #[allow(dead_code)]
    // Used by future: health monitoring feature
    pub async fn health(&self) -> Result<bool> {
        let url = format!("{}/global/health", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send health check request")?;

        if response.status().is_success() {
            let health: HealthResponse = response
                .json()
                .await
                .context("Failed to parse health response")?;
            Ok(health.status == "ok")
        } else {
            Ok(false)
        }
    }

    /// List all sessions
    #[allow(dead_code)]
    // Used by future: session management feature
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let url = format!("{}/sessions", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send list sessions request")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to list sessions: HTTP {}",
                response.status().as_u16()
            );
        }

        let sessions: Vec<SessionInfo> = response
            .json()
            .await
            .context("Failed to parse sessions response")?;

        Ok(sessions)
    }

    /// Get a specific session by ID
    #[allow(dead_code)]
    // Used by future: session lookup feature
    pub async fn get_session(&self, id: &str) -> Result<SessionInfo> {
        let url = format!("{}/session/{}", self.base_url, id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send get session request")?;

        match response.status() {
            StatusCode::OK => {
                let session: SessionInfo = response
                    .json()
                    .await
                    .context("Failed to parse session response")?;
                Ok(session)
            }
            StatusCode::NOT_FOUND => {
                anyhow::bail!("Session not found: {}", id)
            }
            status => {
                anyhow::bail!("Failed to get session: HTTP {}", status.as_u16())
            }
        }
    }

    /// Create a new session
    #[allow(dead_code)]
    // Used by future: session creation feature
    pub async fn create_session(&self, project_path: &Path) -> Result<SessionInfo> {
        let url = format!("{}/session", self.base_url);
        let request_body = CreateSessionRequest {
            project_path: project_path
                .to_str()
                .context("Invalid project path")?
                .to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send create session request")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to create session: HTTP {}",
                response.status().as_u16()
            );
        }

        let session: SessionInfo = response
            .json()
            .await
            .context("Failed to parse create session response")?;

        Ok(session)
    }

    /// Send a message and wait for response (synchronous)
    #[allow(dead_code)]
    // Used by future: synchronous message sending feature
    pub async fn send_message(&self, session_id: &str, text: &str) -> Result<MessageResponse> {
        let url = format!("{}/session/{}/prompt", self.base_url, session_id);

        // Create a proper message structure
        let message = Message {
            role: "user".to_string(),
            content: vec![MessagePart::Text {
                text: text.to_string(),
            }],
        };

        let request_body = CreateMessageRequest {
            message,
            stream: Some(false),
        };

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send message")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to send message: HTTP {}",
                response.status().as_u16()
            );
        }

        let message_response: MessageResponse = response
            .json()
            .await
            .context("Failed to parse message response")?;

        Ok(message_response)
    }

    /// Send a message asynchronously (fire and forget)
    pub async fn send_message_async(&self, session_id: &str, text: &str) -> Result<()> {
        let url = format!("{}/session/{}/prompt_async", self.base_url, session_id);

        // Create a proper message structure
        let message = Message {
            role: "user".to_string(),
            content: vec![MessagePart::Text {
                text: text.to_string(),
            }],
        };

        let request_body = CreateMessageRequest {
            message,
            stream: Some(false),
        };

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send async message")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to send async message: HTTP {}",
                response.status().as_u16()
            );
        }

        Ok(())
    }

    /// Generate SSE subscription URL for a session
    pub fn sse_url(&self, session_id: &str) -> String {
        format!("{}/session/{}/stream", self.base_url, session_id)
    }

    /// Reply to a permission request
    pub async fn reply_permission(
        &self,
        session_id: &str,
        permission_id: &str,
        allow: bool,
    ) -> Result<()> {
        let url = format!(
            "{}/session/{}/permission/{}/reply",
            self.base_url, session_id, permission_id
        );
        let request_body = PermissionReplyRequest { allow };

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send permission reply")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to reply to permission: HTTP {}",
                response.status().as_u16()
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_new_creates_client() {
        let client = OpenCodeClient::new("http://localhost:4100");
        assert_eq!(client.base_url, "http://localhost:4100");
    }

    #[tokio::test]
    async fn test_new_trims_trailing_slash() {
        let client = OpenCodeClient::new("http://localhost:4100/");
        assert_eq!(client.base_url, "http://localhost:4100");
    }

    #[tokio::test]
    async fn test_health_check_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/global/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "ok"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenCodeClient::new(&mock_server.uri());
        let result = client.health().await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_health_check_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/global/health"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let client = OpenCodeClient::new(&mock_server.uri());
        let result = client.health().await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_list_sessions_empty() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/sessions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&mock_server)
            .await;

        let client = OpenCodeClient::new(&mock_server.uri());
        let sessions = client.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_list_sessions_multiple() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/sessions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": "session-1",
                    "title": "Session 1",
                    "created": 1640000000,
                    "updated": 1640000100
                },
                {
                    "id": "session-2",
                    "title": "Session 2",
                    "created": 1640000200,
                    "updated": 1640000300
                }
            ])))
            .mount(&mock_server)
            .await;

        let client = OpenCodeClient::new(&mock_server.uri());
        let sessions = client.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, "session-1");
        assert_eq!(sessions[1].id, "session-2");
    }

    #[tokio::test]
    async fn test_get_session_found() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/session/session-123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "session-123",
                "title": "Test Session",
                "created": 1640000000,
                "updated": 1640000100
            })))
            .mount(&mock_server)
            .await;

        let client = OpenCodeClient::new(&mock_server.uri());
        let session = client.get_session("session-123").await.unwrap();
        assert_eq!(session.id, "session-123");
        assert_eq!(session.title, Some("Test Session".to_string()));
    }

    #[tokio::test]
    async fn test_get_session_not_found() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/session/nonexistent"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let client = OpenCodeClient::new(&mock_server.uri());
        let result = client.get_session("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_create_session() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "new-session",
                "title": "New Session",
                "created": 1640000000,
                "updated": 1640000000
            })))
            .mount(&mock_server)
            .await;

        let client = OpenCodeClient::new(&mock_server.uri());
        let session = client
            .create_session(Path::new("/tmp/test-project"))
            .await
            .unwrap();
        assert_eq!(session.id, "new-session");
    }

    #[tokio::test]
    async fn test_send_message_sync() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/session/session-123/prompt"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "message": {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "text",
                            "text": "Hello back!"
                        }
                    ]
                },
                "metadata": {
                    "id": "msg-123",
                    "role": "assistant",
                    "model": "claude-3-opus"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenCodeClient::new(&mock_server.uri());
        let response = client.send_message("session-123", "Hello").await.unwrap();
        assert_eq!(response.message.role, "assistant");
        assert_eq!(response.metadata.id, "msg-123");
    }

    #[tokio::test]
    async fn test_send_message_async() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/session/session-123/prompt_async"))
            .respond_with(ResponseTemplate::new(202))
            .mount(&mock_server)
            .await;

        let client = OpenCodeClient::new(&mock_server.uri());
        let result = client.send_message_async("session-123", "Hello").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sse_url_generation() {
        let client = OpenCodeClient::new("http://localhost:4100");
        let url = client.sse_url("session-123");
        assert_eq!(url, "http://localhost:4100/session/session-123/stream");
    }

    #[tokio::test]
    async fn test_reply_permission_allow() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path_regex(r"/session/.+/permission/.+/reply"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let client = OpenCodeClient::new(&mock_server.uri());
        let result = client
            .reply_permission("session-123", "perm-456", true)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_reply_permission_deny() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path_regex(r"/session/.+/permission/.+/reply"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let client = OpenCodeClient::new(&mock_server.uri());
        let result = client
            .reply_permission("session-123", "perm-456", false)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_http_error_handling() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/sessions"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let client = OpenCodeClient::new(&mock_server.uri());
        let result = client.list_sessions().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTP 500"));
    }

    #[tokio::test]
    async fn test_invalid_json_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/sessions"))
            .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
            .mount(&mock_server)
            .await;

        let client = OpenCodeClient::new(&mock_server.uri());
        let result = client.list_sessions().await;
        assert!(result.is_err());
    }
}
