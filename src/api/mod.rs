use crate::orchestrator::store::OrchestratorStore;
use crate::types::instance::{InstanceInfo, InstanceState, InstanceType};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

/// Application state shared across handlers
#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub store: OrchestratorStore,
    pub api_key: Option<String>,
}

/// Request payload for registering an external instance
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct RegisterRequest {
    pub project_path: String,
    pub port: u16,
    pub session_id: String,
}

/// Request payload for unregistering an instance
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct UnregisterRequest {
    pub project_path: String,
}

/// Response for status endpoint
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct StatusResponse {
    pub registered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<InstanceInfo>,
}

/// Response for instances list endpoint
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct InstancesResponse {
    pub instances: Vec<InstanceInfo>,
}

/// Health check handler
#[allow(dead_code)]
async fn health() -> impl IntoResponse {
    StatusCode::OK
}

/// Register external instance handler
#[allow(dead_code)]
async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    let instance = InstanceInfo {
        id: format!("external-{}", payload.port),
        state: InstanceState::Running,
        instance_type: InstanceType::External,
        project_path: payload.project_path.clone(),
        port: payload.port,
        pid: None,
        started_at: Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        ),
        stopped_at: None,
    };

    match state
        .store
        .save_instance(&instance, Some(&payload.session_id))
        .await
    {
        Ok(_) => (StatusCode::CREATED, Json(instance)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Unregister instance handler
#[allow(dead_code)]
async fn unregister(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UnregisterRequest>,
) -> impl IntoResponse {
    match state
        .store
        .get_instance_by_path(&payload.project_path)
        .await
    {
        Ok(Some(instance)) => match state.store.delete_instance(&instance.id).await {
            Ok(_) => StatusCode::NO_CONTENT.into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response(),
        },
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Check registration status handler
#[allow(dead_code)]
async fn status(
    State(state): State<Arc<AppState>>,
    Path(mut path): Path<String>,
) -> impl IntoResponse {
    if !path.starts_with('/') {
        path = format!("/{}", path);
    }
    match state.store.get_instance_by_path(&path).await {
        Ok(Some(instance)) => (
            StatusCode::OK,
            Json(StatusResponse {
                registered: true,
                instance: Some(instance),
            }),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(StatusResponse {
                registered: false,
                instance: None,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// List all external instances handler
#[allow(dead_code)]
async fn list_instances(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.store.get_all_instances().await {
        Ok(instances) => {
            let external_instances: Vec<InstanceInfo> = instances
                .into_iter()
                .filter(|i| i.instance_type == InstanceType::External)
                .collect();

            (
                StatusCode::OK,
                Json(InstancesResponse {
                    instances: external_instances,
                }),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// API key authentication middleware
#[allow(dead_code)]
async fn api_key_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Response {
    let Some(expected_key) = &state.api_key else {
        return next.run(request).await;
    };

    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match auth_header {
        Some(key) if key == expected_key => next.run(request).await,
        _ => (StatusCode::UNAUTHORIZED, "Unauthorized").into_response(),
    }
}

/// Create the API router
#[allow(dead_code)]
pub fn create_router(state: AppState) -> Router {
    let state = Arc::new(state);

    Router::new()
        .route("/api/health", get(health))
        .route("/api/register", post(register))
        .route("/api/unregister", post(unregister))
        .route("/api/status/{*path}", get(status))
        .route("/api/instances", get(list_instances))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            api_key_middleware,
        ))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Method, Request};
    use tower::ServiceExt;

    async fn create_test_store() -> OrchestratorStore {
        OrchestratorStore::new(":memory:".as_ref()).await.unwrap()
    }

    async fn create_test_app(api_key: Option<String>) -> Router {
        let store = create_test_store().await;
        let state = AppState { store, api_key };
        create_router(state)
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = create_test_app(None).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_register_instance() {
        let app = create_test_app(None).await;

        let payload = serde_json::json!({
            "projectPath": "/test/project",
            "port": 4096,
            "sessionId": "ses_test123"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/register")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_string(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_unregister_instance() {
        let store = create_test_store().await;
        let state = AppState {
            store: store.clone(),
            api_key: None,
        };

        let instance = InstanceInfo {
            id: "external-4096".to_string(),
            state: InstanceState::Running,
            instance_type: InstanceType::External,
            project_path: "/test/project".to_string(),
            port: 4096,
            pid: None,
            started_at: Some(1234567890),
            stopped_at: None,
        };
        state
            .store
            .save_instance(&instance, Some("ses_test123"))
            .await
            .unwrap();

        let app = create_router(state);

        let payload = serde_json::json!({
            "projectPath": "/test/project"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/unregister")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_string(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_status_endpoint_found() {
        let store = create_test_store().await;
        let state = AppState {
            store: store.clone(),
            api_key: None,
        };

        let instance = InstanceInfo {
            id: "external-4096".to_string(),
            state: InstanceState::Running,
            instance_type: InstanceType::External,
            project_path: "/test/project".to_string(),
            port: 4096,
            pid: None,
            started_at: Some(1234567890),
            stopped_at: None,
        };
        state
            .store
            .save_instance(&instance, Some("ses_test123"))
            .await
            .unwrap();

        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/status//test/project")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_status_endpoint_not_found() {
        let app = create_test_app(None).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/status//nonexistent/path")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_instances() {
        let store = create_test_store().await;
        let state = AppState {
            store: store.clone(),
            api_key: None,
        };

        let instance1 = InstanceInfo {
            id: "external-4096".to_string(),
            state: InstanceState::Running,
            instance_type: InstanceType::External,
            project_path: "/test/project1".to_string(),
            port: 4096,
            pid: None,
            started_at: Some(1234567890),
            stopped_at: None,
        };
        state
            .store
            .save_instance(&instance1, Some("ses_test123"))
            .await
            .unwrap();

        let instance2 = InstanceInfo {
            id: "external-4097".to_string(),
            state: InstanceState::Running,
            instance_type: InstanceType::External,
            project_path: "/test/project2".to_string(),
            port: 4097,
            pid: None,
            started_at: Some(1234567891),
            stopped_at: None,
        };
        state
            .store
            .save_instance(&instance2, Some("ses_test456"))
            .await
            .unwrap();

        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/instances")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_api_key_required() {
        let app = create_test_app(Some("secret-key".to_string())).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/instances")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_api_key_valid() {
        let app = create_test_app(Some("secret-key".to_string())).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/instances")
                    .header(header::AUTHORIZATION, "Bearer secret-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_api_key_invalid() {
        let app = create_test_app(Some("secret-key".to_string())).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/instances")
                    .header(header::AUTHORIZATION, "Bearer wrong-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_cors_headers_present() {
        let app = create_test_app(None).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let headers = response.headers();
        assert!(headers.contains_key("access-control-allow-origin"));
    }
}
