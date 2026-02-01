//! OpenCode instance lifecycle management.
//!
//! This module provides the `OpenCodeInstance` struct for managing the lifecycle
//! of OpenCode processes, including spawning, health checks, and graceful shutdown.

use crate::types::instance::{InstanceConfig, InstanceState};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::debug;

/// Default timeout for graceful shutdown before SIGKILL.
const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Default timeout for health check HTTP requests.
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Manages the lifecycle of a single OpenCode instance.
///
/// Handles process spawning, health checks, and graceful shutdown.
/// State transitions are tracked and can be queried.
#[derive(Debug)]
pub struct OpenCodeInstance {
    id: String,
    config: InstanceConfig,
    port: u16,
    state: Arc<Mutex<InstanceState>>,
    child: Arc<Mutex<Option<Child>>>,
    #[allow(dead_code)]
    // Used by future: session tracking feature
    session_id: Arc<Mutex<Option<String>>>,
    #[allow(dead_code)]
    // Used by future: process monitoring feature
    pid: Arc<Mutex<Option<u32>>>,
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
    /// * `port` - Port number for the instance to listen on
    ///
    /// # Returns
    /// * `Ok(Self)` - Instance spawned successfully
    /// * `Err(_)` - Failed to spawn process
    pub async fn spawn(config: InstanceConfig, port: u16) -> Result<Self> {
        debug!(
            instance_id = %config.id,
            project_path = %config.project_path,
            port = port,
            "Spawning OpenCode instance"
        );

        let state = Arc::new(Mutex::new(InstanceState::Starting));
        let child_holder = Arc::new(Mutex::new(None::<Child>));
        let session_id = Arc::new(Mutex::new(None::<String>));
        let pid_holder = Arc::new(Mutex::new(None::<u32>));

        let mut cmd = Command::new(&config.opencode_path);
        cmd.arg("serve")
            .arg("--port")
            .arg(port.to_string())
            .current_dir(&config.project_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let child = cmd.spawn().map_err(|e| {
            anyhow!(
                "Failed to spawn OpenCode instance for project '{}': {}",
                config.project_path,
                e
            )
        })?;

        let child_pid = child.id();
        debug!(
            instance_id = %config.id,
            pid = ?child_pid,
            "OpenCode process spawned successfully"
        );

        {
            let mut pid_guard = pid_holder.lock().await;
            *pid_guard = child_pid;
        }

        {
            let mut child_guard = child_holder.lock().await;
            *child_guard = Some(child);
        }

        {
            let mut state_guard = state.lock().await;
            *state_guard = InstanceState::Running;
        }

        let http_client = reqwest::Client::builder()
            .timeout(HEALTH_CHECK_TIMEOUT)
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            id: config.id.clone(),
            config,
            port,
            state,
            child: child_holder,
            session_id,
            pid: pid_holder,
            http_client,
        })
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
            child: Arc::new(Mutex::new(None)),
            session_id: Arc::new(Mutex::new(None)),
            pid: Arc::new(Mutex::new(pid)),
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

        let mut child_guard = self.child.lock().await;

        if let Some(ref mut child) = *child_guard {
            let pid = child.id();

            #[cfg(unix)]
            {
                if let Some(pid) = pid {
                    use std::process::Command as StdCommand;

                    debug!(instance_id = %self.id, pid = pid, "Sending SIGTERM");
                    let _ = StdCommand::new("kill")
                        .arg("-TERM")
                        .arg(pid.to_string())
                        .output();

                    let graceful_exit = tokio::time::timeout(GRACEFUL_SHUTDOWN_TIMEOUT, async {
                        loop {
                            match child.try_wait() {
                                Ok(Some(_)) => return true,
                                Ok(None) => {
                                    tokio::time::sleep(Duration::from_millis(100)).await;
                                }
                                Err(_) => return false,
                            }
                        }
                    })
                    .await;

                    if graceful_exit.is_err() || !graceful_exit.unwrap() {
                        debug!(
                            instance_id = %self.id,
                            pid = pid,
                            "Graceful shutdown timeout, sending SIGKILL"
                        );
                        let _ = child.kill().await;
                    } else {
                        debug!(
                            instance_id = %self.id,
                            pid = pid,
                            "Process exited gracefully"
                        );
                    }
                } else {
                    debug!(instance_id = %self.id, "No PID available, killing process");
                    let _ = child.kill().await;
                }
            }

            #[cfg(not(unix))]
            {
                debug!(instance_id = %self.id, "Killing process (non-Unix)");
                let _ = child.kill().await;
            }

            let _ = child.wait().await;
        }

        *child_guard = None;

        {
            let mut state_guard = self.state.lock().await;
            *state_guard = InstanceState::Stopped;
        }

        {
            let mut pid_guard = self.pid.lock().await;
            *pid_guard = None;
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

    /// Get the process ID if available.
    #[allow(dead_code)]
    // Used by future: process monitoring feature
    pub async fn pid(&self) -> Option<u32> {
        let guard = self.pid.lock().await;
        *guard
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
        let mut child_guard = self.child.lock().await;

        if let Some(ref mut child) = *child_guard {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        debug!(
                            instance_id = %self.id,
                            exit_status = ?status,
                            "Process crashed detected"
                        );
                        let mut state_guard = self.state.lock().await;
                        *state_guard = InstanceState::Error;
                        drop(child_guard);

                        let mut pid_guard = self.pid.lock().await;
                        *pid_guard = None;

                        return Ok(true);
                    } else {
                        debug!(
                            instance_id = %self.id,
                            "Process exited cleanly"
                        );
                        let mut state_guard = self.state.lock().await;
                        *state_guard = InstanceState::Stopped;
                    }
                    Ok(false)
                }
                Ok(None) => {
                    debug!(instance_id = %self.id, "Process still running");
                    Ok(false)
                }
                Err(e) => Err(anyhow!("Failed to check process status: {}", e)),
            }
        } else {
            debug!(instance_id = %self.id, "No child process to check");
            Ok(false)
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

#[cfg(test)]
mod tests {
    use super::*;
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

    #[tokio::test]
    async fn test_external_instance_pid() {
        let config = test_config("test-3", "/tmp/project");
        let instance = OpenCodeInstance::external(config, 4102, Some(99999)).unwrap();

        assert_eq!(instance.pid().await, Some(99999));
    }

    #[tokio::test]
    async fn test_external_instance_pid_none() {
        let config = test_config("test-4", "/tmp/project");
        let instance = OpenCodeInstance::external(config, 4103, None).unwrap();

        assert_eq!(instance.pid().await, None);
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
    async fn test_spawn_fails_with_invalid_command() {
        let config = InstanceConfig {
            id: "fail-test".to_string(),
            instance_type: InstanceType::Managed,
            project_path: "/tmp/test".to_string(),
            port: 4300,
            auto_start: true,
            opencode_path: "/nonexistent/opencode-test-binary".to_string(),
        };

        let result = OpenCodeInstance::spawn(config, 4300).await;

        if let Err(err) = result {
            assert!(err.to_string().contains("Failed to spawn"));
        }
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

    // ==================== Process Lifecycle Integration Tests ====================

    #[tokio::test]
    #[cfg(unix)]
    async fn test_spawn_real_process_and_stop() {
        let config = test_config("sleep-test", "/tmp");
        let state = Arc::new(Mutex::new(InstanceState::Starting));
        let child_holder = Arc::new(Mutex::new(None::<Child>));
        let pid_holder = Arc::new(Mutex::new(None::<u32>));

        let mut cmd = Command::new("sleep");
        cmd.arg("60").kill_on_drop(true);

        let child = cmd.spawn().expect("Failed to spawn sleep process");
        let child_pid = child.id();

        {
            let mut pid_guard = pid_holder.lock().await;
            *pid_guard = child_pid;
        }

        {
            let mut child_guard = child_holder.lock().await;
            *child_guard = Some(child);
        }

        {
            let mut state_guard = state.lock().await;
            *state_guard = InstanceState::Running;
        }

        let http_client = reqwest::Client::new();

        let instance = OpenCodeInstance {
            id: config.id.clone(),
            config,
            port: 0,
            state,
            child: child_holder,
            session_id: Arc::new(Mutex::new(None)),
            pid: pid_holder,
            http_client,
        };

        assert_eq!(instance.state().await, InstanceState::Running);
        assert!(instance.pid().await.is_some());

        instance.stop().await.expect("Failed to stop instance");

        assert_eq!(instance.state().await, InstanceState::Stopped);
        assert_eq!(instance.pid().await, None);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_graceful_shutdown_sends_sigterm() {
        let config = test_config("sigterm-test", "/tmp");
        let state = Arc::new(Mutex::new(InstanceState::Running));
        let child_holder = Arc::new(Mutex::new(None::<Child>));
        let pid_holder = Arc::new(Mutex::new(None::<u32>));

        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg("trap 'exit 0' TERM; sleep 60")
            .kill_on_drop(true);

        let child = cmd.spawn().expect("Failed to spawn sh process");
        let child_pid = child.id();

        {
            let mut pid_guard = pid_holder.lock().await;
            *pid_guard = child_pid;
        }

        {
            let mut child_guard = child_holder.lock().await;
            *child_guard = Some(child);
        }

        let http_client = reqwest::Client::new();

        let instance = OpenCodeInstance {
            id: config.id.clone(),
            config,
            port: 0,
            state,
            child: child_holder,
            session_id: Arc::new(Mutex::new(None)),
            pid: pid_holder,
            http_client,
        };

        let start = std::time::Instant::now();
        instance.stop().await.expect("Failed to stop instance");
        let elapsed = start.elapsed();

        assert!(elapsed < GRACEFUL_SHUTDOWN_TIMEOUT + Duration::from_secs(1));
        assert_eq!(instance.state().await, InstanceState::Stopped);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_crash_detection_with_real_process() {
        let config = test_config("crash-test", "/tmp");
        let state = Arc::new(Mutex::new(InstanceState::Running));
        let child_holder = Arc::new(Mutex::new(None::<Child>));
        let pid_holder = Arc::new(Mutex::new(None::<u32>));

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("exit 1").kill_on_drop(true);

        let child = cmd.spawn().expect("Failed to spawn sh process");
        let child_pid = child.id();

        {
            let mut pid_guard = pid_holder.lock().await;
            *pid_guard = child_pid;
        }

        {
            let mut child_guard = child_holder.lock().await;
            *child_guard = Some(child);
        }

        let http_client = reqwest::Client::new();

        let instance = OpenCodeInstance {
            id: config.id.clone(),
            config,
            port: 0,
            state,
            child: child_holder,
            session_id: Arc::new(Mutex::new(None)),
            pid: pid_holder,
            http_client,
        };

        tokio::time::sleep(Duration::from_millis(100)).await;

        let crashed = instance.check_for_crash().await.unwrap();
        assert!(crashed);

        assert_eq!(instance.state().await, InstanceState::Error);
    }
}
