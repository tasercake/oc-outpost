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
use tracing::debug;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub store: OrchestratorStore,
    pub api_key: Option<String>,
}

/// Request payload for registering an external instance
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub project_path: String,
    pub port: u16,
    pub session_id: String,
}

/// Request payload for unregistering an instance
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnregisterRequest {
    pub project_path: String,
}

/// Response for status endpoint
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub registered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<InstanceInfo>,
}

/// Response for instances list endpoint
#[derive(Debug, Serialize)]
pub struct InstancesResponse {
    pub instances: Vec<InstanceInfo>,
}

/// Health check handler
async fn health() -> impl IntoResponse {
    debug!(method = "GET", path = "/api/health", "API request received");
    let status = StatusCode::OK;
    debug!(status = status.as_u16(), "API response");
    status
}

/// Register external instance handler
async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    debug!(
        method = "POST",
        path = "/api/register",
        "API request received"
    );
    debug!(port = payload.port, project_path = %payload.project_path, "Registering instance");

    let instance = InstanceInfo {
        id: format!("external-{}", payload.port),
        state: InstanceState::Running,
        instance_type: InstanceType::External,
        project_path: payload.project_path.clone(),
        port: payload.port,
        pid: None,
        container_id: None,
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
        Ok(_) => {
            debug!(instance_id = %instance.id, status = StatusCode::CREATED.as_u16(), "API response");
            (StatusCode::CREATED, Json(instance)).into_response()
        }
        Err(e) => {
            debug!(error = %e, status = StatusCode::INTERNAL_SERVER_ERROR.as_u16(), "API response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

/// Unregister instance handler
async fn unregister(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UnregisterRequest>,
) -> impl IntoResponse {
    debug!(
        method = "POST",
        path = "/api/unregister",
        "API request received"
    );
    debug!(project_path = %payload.project_path, "Unregistering instance");

    match state
        .store
        .get_instance_by_path(&payload.project_path)
        .await
    {
        Ok(Some(instance)) => match state.store.delete_instance(&instance.id).await {
            Ok(_) => {
                debug!(instance_id = %instance.id, status = StatusCode::NO_CONTENT.as_u16(), "API response");
                StatusCode::NO_CONTENT.into_response()
            }
            Err(e) => {
                debug!(error = %e, status = StatusCode::INTERNAL_SERVER_ERROR.as_u16(), "API response");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
                    .into_response()
            }
        },
        Ok(None) => {
            debug!(status = StatusCode::NOT_FOUND.as_u16(), "API response");
            StatusCode::NOT_FOUND.into_response()
        }
        Err(e) => {
            debug!(error = %e, status = StatusCode::INTERNAL_SERVER_ERROR.as_u16(), "API response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

/// Check registration status handler
async fn status(
    State(state): State<Arc<AppState>>,
    Path(mut path): Path<String>,
) -> impl IntoResponse {
    debug!(
        method = "GET",
        path = "/api/status/{*path}",
        "API request received"
    );

    if !path.starts_with('/') {
        path = format!("/{}", path);
    }
    debug!(project_path = %path, "Checking instance status");

    match state.store.get_instance_by_path(&path).await {
        Ok(Some(instance)) => {
            debug!(instance_id = %instance.id, status = StatusCode::OK.as_u16(), "API response");
            (
                StatusCode::OK,
                Json(StatusResponse {
                    registered: true,
                    instance: Some(instance),
                }),
            )
                .into_response()
        }
        Ok(None) => {
            debug!(status = StatusCode::NOT_FOUND.as_u16(), "API response");
            (
                StatusCode::NOT_FOUND,
                Json(StatusResponse {
                    registered: false,
                    instance: None,
                }),
            )
                .into_response()
        }
        Err(e) => {
            debug!(error = %e, status = StatusCode::INTERNAL_SERVER_ERROR.as_u16(), "API response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

/// List all external instances handler
async fn list_instances(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    debug!(
        method = "GET",
        path = "/api/instances",
        "API request received"
    );

    match state.store.get_all_instances().await {
        Ok(instances) => {
            let external_instances: Vec<InstanceInfo> = instances
                .into_iter()
                .filter(|i| i.instance_type == InstanceType::External)
                .collect();

            let count = external_instances.len();
            debug!(
                count = count,
                status = StatusCode::OK.as_u16(),
                "Listing instances"
            );
            debug!(status = StatusCode::OK.as_u16(), "API response");

            (
                StatusCode::OK,
                Json(InstancesResponse {
                    instances: external_instances,
                }),
            )
                .into_response()
        }
        Err(e) => {
            debug!(error = %e, status = StatusCode::INTERNAL_SERVER_ERROR.as_u16(), "API response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

/// API key authentication middleware
async fn api_key_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Response {
    let has_api_key = state.api_key.is_some();
    debug!(has_api_key = has_api_key, "API auth check");

    let Some(expected_key) = &state.api_key else {
        debug!(authenticated = true, "API auth check - no key required");
        return next.run(request).await;
    };

    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match auth_header {
        Some(key) if key == expected_key => {
            debug!(authenticated = true, "API auth check");
            next.run(request).await
        }
        _ => {
            debug!(authenticated = false, "API auth check");
            (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
        }
    }
}

/// Create the API router
pub fn create_router(state: AppState) -> Router {
    debug!("Creating API router");
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
            container_id: None,
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
            container_id: None,
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
            container_id: None,
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
            container_id: None,
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
