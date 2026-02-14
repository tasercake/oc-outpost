//! OpenCode instance lifecycle management.
//!
//! This module provides the `OpenCodeInstance` struct for managing the lifecycle
//! of OpenCode processes, including spawning, health checks, and graceful shutdown.

use crate::orchestrator::container::{ContainerConfig, ContainerRuntime, ContainerState};
use crate::types::instance::{InstanceConfig, InstanceState};
use anyhow::{anyhow, Result};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::debug;

/// Default timeout for graceful shutdown before SIGKILL.
const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Default timeout for health check HTTP requests.
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Manages the lifecycle of a single OpenCode instance.
///
/// Handles container spawning, health checks, and graceful shutdown.
/// State transitions are tracked and can be queried.
pub struct OpenCodeInstance {
    id: String,
    config: InstanceConfig,
    port: u16,
    state: Arc<Mutex<InstanceState>>,
    runtime: Option<Arc<dyn ContainerRuntime>>,
    container_id: Arc<Mutex<Option<String>>>,
    #[allow(dead_code)]
    // Used by future: session tracking feature
    session_id: Arc<Mutex<Option<String>>>,
    http_client: reqwest::Client,
}

impl OpenCodeInstance {
    /// Spawn a new OpenCode instance with the given configuration and port.
    ///
    /// The instance will be started with `opencode serve --port PORT --project PATH`.
    /// Initial state is `Starting`, transitioning to `Running` after successful spawn.
    ///
    /// # Arguments
    /// * `config` - Instance configuration including project path
    /// * `runtime` - Container runtime implementation
    /// * `container_config` - Container configuration
    ///
    /// # Returns
    /// * `Ok(Self)` - Instance spawned successfully
    /// * `Err(_)` - Failed to spawn process
    pub async fn spawn(
        config: InstanceConfig,
        port: u16,
        runtime: Arc<dyn ContainerRuntime>,
        container_config: ContainerConfig,
    ) -> Result<(Self, String)> {
        debug!(
            instance_id = %config.id,
            project_path = %config.project_path,
            port = port,
            "Spawning OpenCode instance"
        );

        let state = Arc::new(Mutex::new(InstanceState::Starting));
        let container_id_holder = Arc::new(Mutex::new(None::<String>));
        let session_id = Arc::new(Mutex::new(None::<String>));

        let container_id = runtime
            .create_container(&container_config)
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to create container for project '{}': {}",
                    config.project_path,
                    e
                )
            })?;
        runtime.start_container(&container_id).await.map_err(|e| {
            anyhow!(
                "Failed to start container for project '{}': {}",
                config.project_path,
                e
            )
        })?;

        debug!(
            instance_id = %config.id,
            container_id = %container_id,
            "OpenCode container started successfully"
        );

        {
            let mut container_guard = container_id_holder.lock().await;
            *container_guard = Some(container_id.clone());
        }

        {
            let mut state_guard = state.lock().await;
            *state_guard = InstanceState::Running;
        }

        let http_client = reqwest::Client::builder()
            .timeout(HEALTH_CHECK_TIMEOUT)
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        Ok((
            Self {
                id: config.id.clone(),
                config,
                port,
                state,
                runtime: Some(runtime),
                container_id: container_id_holder,
                session_id,
                http_client,
            },
            container_id,
        ))
    }

    /// Create an instance for an external process (not spawned by us).
    ///
    /// Useful for discovered or externally registered instances.
    ///
    /// # Arguments
    /// * `config` - Instance configuration
    /// * `port` - Port the external instance is listening on
    /// * `pid` - Optional PID of the external process
    pub fn external(config: InstanceConfig, port: u16, pid: Option<u32>) -> Result<Self> {
        debug!(
            instance_id = %config.id,
            port = port,
            pid = ?pid,
            "Creating external instance"
        );

        let http_client = reqwest::Client::builder()
            .timeout(HEALTH_CHECK_TIMEOUT)
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            id: config.id.clone(),
            config,
            port,
            state: Arc::new(Mutex::new(InstanceState::Running)),
            runtime: None,
            container_id: Arc::new(Mutex::new(None)),
            session_id: Arc::new(Mutex::new(None)),
            http_client,
        })
    }

    /// Perform a health check by polling the instance's health endpoint.
    ///
    /// Sends a GET request to `http://localhost:{port}/global/health`.
    ///
    /// # Returns
    /// * `Ok(true)` - Instance is healthy
    /// * `Ok(false)` - Health check failed (instance not ready or unhealthy)
    /// * `Err(_)` - HTTP request failed
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("http://localhost:{}/global/health", self.port);
        debug!(instance_id = %self.id, url = %url, "Checking instance health");

        match self.http_client.get(&url).send().await {
            Ok(response) => {
                let is_healthy = response.status().is_success();
                debug!(
                    instance_id = %self.id,
                    healthy = is_healthy,
                    status = response.status().as_u16(),
                    "Health check result"
                );
                Ok(is_healthy)
            }
            Err(e) if e.is_connect() || e.is_timeout() => {
                debug!(
                    instance_id = %self.id,
                    reason = "connection timeout",
                    "Health check failed"
                );
                Ok(false)
            }
            Err(e) => {
                debug!(
                    instance_id = %self.id,
                    error = %e,
                    "Health check error"
                );
                Err(anyhow!("Health check failed: {}", e))
            }
        }
    }

    /// Stop the instance gracefully.
    ///
    /// 1. Sends SIGTERM to the process
    /// 2. Waits up to 5 seconds for graceful exit
    /// 3. If still running, sends SIGKILL
    ///
    /// # Returns
    /// * `Ok(())` - Instance stopped successfully
    /// * `Err(_)` - Failed to stop instance
    pub async fn stop(&self) -> Result<()> {
        debug!(instance_id = %self.id, "Stopping instance");

        {
            let mut state_guard = self.state.lock().await;
            *state_guard = InstanceState::Stopping;
        }

        let runtime = self.runtime.as_ref().map(Arc::clone);
        let container_id = { self.container_id.lock().await.clone() };

        if let (Some(runtime), Some(container_id)) = (runtime, container_id) {
            runtime
                .stop_container(&container_id, GRACEFUL_SHUTDOWN_TIMEOUT.as_secs())
                .await?;
            runtime.remove_container(&container_id, true).await?;
        }

        {
            let mut state_guard = self.state.lock().await;
            *state_guard = InstanceState::Stopped;
        }

        {
            let mut container_guard = self.container_id.lock().await;
            *container_guard = None;
        }

        debug!(instance_id = %self.id, "Instance stopped");
        Ok(())
    }

    /// Get the current state of the instance.
    pub async fn state(&self) -> InstanceState {
        let state_guard = self.state.lock().await;
        state_guard.clone()
    }

    /// Get the port number this instance is listening on.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get the project path for this instance.
    pub fn project_path(&self) -> &str {
        &self.config.project_path
    }

    /// Get the instance ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the session ID if set.
    #[allow(dead_code)]
    // Used by future: session tracking feature
    pub async fn session_id(&self) -> Option<String> {
        let guard = self.session_id.lock().await;
        guard.clone()
    }

    /// Set the session ID.
    #[allow(dead_code)]
    // Used by future: session tracking feature
    pub async fn set_session_id(&self, session_id: Option<String>) {
        let mut guard = self.session_id.lock().await;
        *guard = session_id;
    }

    /// Set the state (for external state updates like crash detection).
    #[allow(dead_code)]
    // Used by future: state management feature
    pub async fn set_state(&self, new_state: InstanceState) {
        let mut guard = self.state.lock().await;
        *guard = new_state;
    }

    /// Check if the process has crashed (exited unexpectedly).
    ///
    /// Returns true if the process was running but has now exited.
    /// Updates state to Error if crash detected.
    pub async fn check_for_crash(&self) -> Result<bool> {
        let runtime = self.runtime.as_ref().map(Arc::clone);
        let container_id = { self.container_id.lock().await.clone() };

        let (runtime, container_id) = match (runtime, container_id) {
            (Some(runtime), Some(container_id)) => (runtime, container_id),
            _ => {
                debug!(instance_id = %self.id, "No container to check");
                return Ok(false);
            }
        };

        let info = runtime.inspect_container(&container_id).await?;
        match info.state {
            ContainerState::Running => {
                debug!(instance_id = %self.id, "Container still running");
                Ok(false)
            }
            ContainerState::Exited(code) => {
                let mut state_guard = self.state.lock().await;
                if code == 0 {
                    debug!(
                        instance_id = %self.id,
                        exit_code = code,
                        "Container exited cleanly"
                    );
                    *state_guard = InstanceState::Stopped;
                } else {
                    debug!(
                        instance_id = %self.id,
                        exit_code = code,
                        "Container crash detected"
                    );
                    *state_guard = InstanceState::Error;
                }
                drop(state_guard);

                let mut container_guard = self.container_id.lock().await;
                *container_guard = None;

                Ok(code != 0)
            }
            ContainerState::Created | ContainerState::Unknown(_) => Ok(false),
        }
    }

    /// Wait for the instance to become ready (health check succeeds).
    ///
    /// Polls the health endpoint with the given interval until timeout.
    ///
    /// # Arguments
    /// * `timeout` - Maximum time to wait for readiness
    /// * `poll_interval` - Time between health check attempts
    ///
    /// # Returns
    /// * `Ok(true)` - Instance became ready
    /// * `Ok(false)` - Timeout reached without instance becoming ready
    pub async fn wait_for_ready(&self, timeout: Duration, poll_interval: Duration) -> Result<bool> {
        let start = std::time::Instant::now();
        let mut poll_count = 0;

        debug!(
            instance_id = %self.id,
            timeout_secs = timeout.as_secs(),
            poll_interval_ms = poll_interval.as_millis(),
            "Starting readiness polling"
        );

        while start.elapsed() < timeout {
            poll_count += 1;
            debug!(
                instance_id = %self.id,
                poll_number = poll_count,
                elapsed_ms = start.elapsed().as_millis(),
                "Polling instance readiness"
            );

            if self.health_check().await? {
                debug!(
                    instance_id = %self.id,
                    poll_count = poll_count,
                    total_wait_ms = start.elapsed().as_millis(),
                    "Instance became ready"
                );
                return Ok(true);
            }
            tokio::time::sleep(poll_interval).await;
        }

        debug!(
            instance_id = %self.id,
            poll_count = poll_count,
            timeout_ms = timeout.as_millis(),
            "Instance readiness timeout"
        );
        Ok(false)
    }
}

impl fmt::Debug for OpenCodeInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenCodeInstance")
            .field("id", &self.id)
            .field("config", &self.config)
            .field("port", &self.port)
            .field("state", &"Mutex<InstanceState>")
            .field("container_id", &"Mutex<Option<String>>")
            .field("session_id", &"Mutex<Option<String>>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::container::mock::{MockAction, MockRuntime};
    use crate::orchestrator::container::{ContainerConfig, ContainerInfo, ContainerState};
    use crate::types::instance::InstanceType;

    /// Helper to create a test InstanceConfig
    fn test_config(id: &str, project_path: &str) -> InstanceConfig {
        InstanceConfig {
            id: id.to_string(),
            instance_type: InstanceType::Managed,
            project_path: project_path.to_string(),
            port: 0,
            auto_start: true,
            opencode_path: "opencode".to_string(),
        }
    }

    fn test_container_config(instance_id: &str, host_port: u16) -> ContainerConfig {
        ContainerConfig {
            instance_id: instance_id.to_string(),
            image: "ghcr.io/sst/opencode".to_string(),
            host_port,
            container_port: 8080,
            worktree_path: "/tmp/project".to_string(),
            config_mount_path: "/tmp/config".to_string(),
            env_vars: vec![],
        }
    }

    // ==================== External Instance Tests ====================

    #[tokio::test]
    async fn test_external_creates_instance() {
        let config = test_config("test-1", "/tmp/test-project");
        let instance = OpenCodeInstance::external(config, 4100, Some(12345)).unwrap();

        assert_eq!(instance.id(), "test-1");
        assert_eq!(instance.port(), 4100);
        assert_eq!(instance.project_path(), "/tmp/test-project");
    }

    #[tokio::test]
    async fn test_external_instance_state_is_running() {
        let config = test_config("test-2", "/tmp/project");
        let instance = OpenCodeInstance::external(config, 4101, None).unwrap();

        assert_eq!(instance.state().await, InstanceState::Running);
    }

    // ==================== Port Getter Tests ====================

    #[tokio::test]
    async fn test_port_getter() {
        let config = test_config("test-5", "/tmp/project");
        let instance = OpenCodeInstance::external(config, 4200, None).unwrap();

        assert_eq!(instance.port(), 4200);
    }

    // ==================== Project Path Getter Tests ====================

    #[tokio::test]
    async fn test_project_path_getter() {
        let config = test_config("test-6", "/home/user/my-project");
        let instance = OpenCodeInstance::external(config, 4201, None).unwrap();

        assert_eq!(instance.project_path(), "/home/user/my-project");
    }

    // ==================== Session ID Tests ====================

    #[tokio::test]
    async fn test_session_id_getter_initial_none() {
        let config = test_config("test-7", "/tmp/project");
        let instance = OpenCodeInstance::external(config, 4202, None).unwrap();

        assert_eq!(instance.session_id().await, None);
    }

    #[tokio::test]
    async fn test_session_id_set_and_get() {
        let config = test_config("test-8", "/tmp/project");
        let instance = OpenCodeInstance::external(config, 4203, None).unwrap();

        instance
            .set_session_id(Some("session-123".to_string()))
            .await;

        assert_eq!(instance.session_id().await, Some("session-123".to_string()));
    }

    // ==================== State Transition Tests ====================

    #[tokio::test]
    async fn test_state_transitions_correctly() {
        let config = test_config("test-9", "/tmp/project");
        let instance = OpenCodeInstance::external(config, 4204, None).unwrap();

        assert_eq!(instance.state().await, InstanceState::Running);

        instance.set_state(InstanceState::Stopping).await;
        assert_eq!(instance.state().await, InstanceState::Stopping);

        instance.set_state(InstanceState::Stopped).await;
        assert_eq!(instance.state().await, InstanceState::Stopped);
    }

    #[tokio::test]
    async fn test_state_can_transition_to_error() {
        let config = test_config("test-10", "/tmp/project");
        let instance = OpenCodeInstance::external(config, 4205, None).unwrap();

        instance.set_state(InstanceState::Error).await;
        assert_eq!(instance.state().await, InstanceState::Error);
    }

    // ==================== Health Check Tests ====================

    #[tokio::test]
    async fn test_health_check_fails_when_not_running() {
        let config = test_config("test-11", "/tmp/project");
        let instance = OpenCodeInstance::external(config, 59999, None).unwrap();

        let healthy = instance.health_check().await.unwrap();
        assert!(!healthy);
    }

    // ==================== Crash Detection Tests ====================

    #[tokio::test]
    async fn test_crash_detection_no_child() {
        let config = test_config("test-12", "/tmp/project");
        let instance = OpenCodeInstance::external(config, 4206, None).unwrap();

        let crashed = instance.check_for_crash().await.unwrap();
        assert!(!crashed);
    }

    // ==================== Stop Tests ====================

    #[tokio::test]
    async fn test_stop_external_instance() {
        let config = test_config("test-13", "/tmp/project");
        let instance = OpenCodeInstance::external(config, 4207, None).unwrap();

        instance.stop().await.unwrap();

        assert_eq!(instance.state().await, InstanceState::Stopped);
    }

    // ==================== Spawn Tests ====================

    #[tokio::test]
    async fn test_spawn_with_mock_runtime() {
        let mut config = test_config("spawn-test", "/tmp/project");
        config.port = 4300;
        let container_config = test_container_config("spawn-test", 4300);
        let runtime = Arc::new(MockRuntime::new());
        let runtime_arc: Arc<dyn ContainerRuntime> = runtime.clone();

        let (instance, container_id) =
            OpenCodeInstance::spawn(config, 4300, runtime_arc, container_config)
                .await
                .unwrap();

        assert_eq!(instance.state().await, InstanceState::Running);
        assert_eq!(container_id, "mock-container-id-abc123".to_string());

        let actions = runtime.recorded_actions();
        assert!(matches!(actions[0], MockAction::CreateContainer { .. }));
        assert!(matches!(actions[1], MockAction::StartContainer { .. }));
    }

    #[tokio::test]
    async fn test_stop_with_mock_runtime() {
        let mut config = test_config("stop-test", "/tmp/project");
        config.port = 4301;
        let container_config = test_container_config("stop-test", 4301);
        let runtime = Arc::new(MockRuntime::new());
        let runtime_arc: Arc<dyn ContainerRuntime> = runtime.clone();

        let (instance, _) = OpenCodeInstance::spawn(config, 4301, runtime_arc, container_config)
            .await
            .unwrap();

        instance.stop().await.unwrap();

        assert_eq!(instance.state().await, InstanceState::Stopped);

        let actions = runtime.recorded_actions();
        assert!(matches!(actions[2], MockAction::StopContainer { .. }));
        assert!(matches!(actions[3], MockAction::RemoveContainer { .. }));
    }

    #[tokio::test]
    async fn test_crash_detection_running() {
        let mut config = test_config("running-test", "/tmp/project");
        config.port = 4302;
        let container_config = test_container_config("running-test", 4302);
        let runtime = Arc::new(MockRuntime::new().with_inspect_result(Ok(ContainerInfo {
            id: "mock-container-id-abc123".to_string(),
            name: "oc-running-test".to_string(),
            state: ContainerState::Running,
        })));
        let runtime_arc: Arc<dyn ContainerRuntime> = runtime.clone();

        let (instance, _) = OpenCodeInstance::spawn(config, 4302, runtime_arc, container_config)
            .await
            .unwrap();

        let crashed = instance.check_for_crash().await.unwrap();
        assert!(!crashed);
        assert_eq!(instance.state().await, InstanceState::Running);
    }

    #[tokio::test]
    async fn test_crash_detection_exited_nonzero() {
        let mut config = test_config("crash-test", "/tmp/project");
        config.port = 4303;
        let container_config = test_container_config("crash-test", 4303);
        let runtime = Arc::new(MockRuntime::new().with_inspect_result(Ok(ContainerInfo {
            id: "mock-container-id-abc123".to_string(),
            name: "oc-crash-test".to_string(),
            state: ContainerState::Exited(1),
        })));
        let runtime_arc: Arc<dyn ContainerRuntime> = runtime.clone();

        let (instance, _) = OpenCodeInstance::spawn(config, 4303, runtime_arc, container_config)
            .await
            .unwrap();

        let crashed = instance.check_for_crash().await.unwrap();
        assert!(crashed);
        assert_eq!(instance.state().await, InstanceState::Error);
    }

    #[tokio::test]
    async fn test_crash_detection_exited_zero() {
        let mut config = test_config("exit-zero-test", "/tmp/project");
        config.port = 4304;
        let container_config = test_container_config("exit-zero-test", 4304);
        let runtime = Arc::new(MockRuntime::new().with_inspect_result(Ok(ContainerInfo {
            id: "mock-container-id-abc123".to_string(),
            name: "oc-exit-zero-test".to_string(),
            state: ContainerState::Exited(0),
        })));
        let runtime_arc: Arc<dyn ContainerRuntime> = runtime.clone();

        let (instance, _) = OpenCodeInstance::spawn(config, 4304, runtime_arc, container_config)
            .await
            .unwrap();

        let crashed = instance.check_for_crash().await.unwrap();
        assert!(!crashed);
        assert_eq!(instance.state().await, InstanceState::Stopped);
    }

    #[tokio::test]
    async fn test_spawn_failure_propagates_error() {
        let mut config = test_config("fail-test", "/tmp/project");
        config.port = 4305;
        let container_config = test_container_config("fail-test", 4305);
        let runtime =
            Arc::new(MockRuntime::new().with_create_result(Err("image not found".to_string())));
        let runtime_arc: Arc<dyn ContainerRuntime> = runtime.clone();

        let result = OpenCodeInstance::spawn(config, 4305, runtime_arc, container_config).await;

        let err = result.unwrap_err();
        assert!(err.to_string().contains("Failed to create container"));

        let actions = runtime.recorded_actions();
        assert!(matches!(actions[0], MockAction::CreateContainer { .. }));
        assert_eq!(actions.len(), 1);
    }

    // ==================== Wait for Ready Tests ====================

    #[tokio::test]
    async fn test_wait_for_ready_timeout() {
        let config = test_config("test-14", "/tmp/project");
        let instance = OpenCodeInstance::external(config, 59998, None).unwrap();

        let ready = instance
            .wait_for_ready(Duration::from_millis(200), Duration::from_millis(50))
            .await
            .unwrap();

        assert!(!ready);
    }

    // ==================== Multiple Concurrent Instances ====================

    #[tokio::test]
    async fn test_multiple_instances_have_unique_ports() {
        let config1 = test_config("instance-1", "/tmp/project1");
        let config2 = test_config("instance-2", "/tmp/project2");
        let config3 = test_config("instance-3", "/tmp/project3");

        let instance1 = OpenCodeInstance::external(config1, 4300, None).unwrap();
        let instance2 = OpenCodeInstance::external(config2, 4301, None).unwrap();
        let instance3 = OpenCodeInstance::external(config3, 4302, None).unwrap();

        assert_ne!(instance1.port(), instance2.port());
        assert_ne!(instance2.port(), instance3.port());
        assert_ne!(instance1.port(), instance3.port());

        assert_ne!(instance1.id(), instance2.id());
        assert_ne!(instance2.id(), instance3.id());
    }
}
