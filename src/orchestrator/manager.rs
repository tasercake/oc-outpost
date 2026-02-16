//! InstanceManager - Coordinates lifecycle of all OpenCode instances.
//!
//! Responsibilities:
//! - Instance lifecycle coordination (create, get, stop)
//! - Resource limits (max instances from config)
//! - Auto-restart with exponential backoff
//! - Periodic health checks
//! - Idle timeout handling
//! - Integration with OrchestratorStore for persistence
//! - Integration with PortPool for port allocation

use crate::config::Config;
use crate::orchestrator::container::{ContainerConfig, ContainerRuntime};
use crate::orchestrator::instance::OpenCodeInstance;
use crate::orchestrator::port_pool::PortPool;
use crate::orchestrator::store::OrchestratorStore;
use crate::types::instance::{InstanceConfig, InstanceInfo, InstanceState};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::debug;

/// Maximum number of restart attempts before giving up.
const MAX_RESTART_ATTEMPTS: usize = 5;

/// Initial restart delay (doubles each attempt: 1s, 2s, 4s, 8s, 16s).
const INITIAL_RESTART_DELAY: Duration = Duration::from_secs(1);

/// Status information for the InstanceManager.
#[derive(Debug, Clone)]
pub struct ManagerStatus {
    #[allow(dead_code)]
    // Used by future: detailed status reporting feature
    pub total_instances: usize,
    #[allow(dead_code)]
    // Used by future: detailed status reporting feature
    pub running_instances: usize,
    #[allow(dead_code)]
    // Used by future: detailed status reporting feature
    pub stopped_instances: usize,
    #[allow(dead_code)]
    // Used by future: detailed status reporting feature
    pub error_instances: usize,
    pub available_ports: usize,
}

/// Tracks restart attempts for auto-restart with backoff.
#[derive(Debug, Clone, Default)]
struct RestartTracker {
    attempt: usize,
    last_attempt: Option<Instant>,
}

/// Tracks activity for idle timeout handling.
#[derive(Debug, Clone)]
struct ActivityTracker {
    last_activity: Instant,
}

impl Default for ActivityTracker {
    fn default() -> Self {
        Self {
            last_activity: Instant::now(),
        }
    }
}

/// Manages the lifecycle of all OpenCode instances.
///
/// Provides:
/// - get_or_create: Get existing instance or spawn new one
/// - Resource limits: Enforces max instances from config
/// - Auto-restart: Restarts crashed instances with exponential backoff
/// - Health checks: Periodic health monitoring via background task
/// - Persistence: Integrates with OrchestratorStore
pub struct InstanceManager {
    config: Arc<Config>,
    runtime: Arc<dyn ContainerRuntime>,
    store: Arc<Mutex<OrchestratorStore>>,
    port_pool: Arc<PortPool>,
    instances: Arc<Mutex<HashMap<String, Arc<Mutex<OpenCodeInstance>>>>>,
    restart_trackers: Arc<Mutex<HashMap<String, RestartTracker>>>,
    activity_trackers: Arc<Mutex<HashMap<String, ActivityTracker>>>,
    shutdown_signal: Arc<Mutex<bool>>,
}

impl InstanceManager {
    /// Create a new InstanceManager.
    ///
    /// # Arguments
    /// * `config` - Application configuration
    /// * `store` - OrchestratorStore for persistence
    /// * `port_pool` - PortPool for port allocation
    pub async fn new(
        config: Arc<Config>,
        store: OrchestratorStore,
        port_pool: PortPool,
        runtime: Arc<dyn ContainerRuntime>,
    ) -> Result<Self> {
        Ok(Self {
            config,
            runtime,
            store: Arc::new(Mutex::new(store)),
            port_pool: Arc::new(port_pool),
            instances: Arc::new(Mutex::new(HashMap::new())),
            restart_trackers: Arc::new(Mutex::new(HashMap::new())),
            activity_trackers: Arc::new(Mutex::new(HashMap::new())),
            shutdown_signal: Arc::new(Mutex::new(false)),
        })
    }

    /// Get an existing instance or create a new one for the given project path.
    ///
    /// Logic:
    /// 1. Check if instance exists by path
    /// 2. If exists and running, return it
    /// 3. If exists but stopped, restart it
    /// 4. If not exists, allocate port and spawn new
    /// 5. Save to database
    pub async fn get_or_create(
        &self,
        project_path: &Path,
        topic_id: i32,
    ) -> Result<Arc<Mutex<OpenCodeInstance>>> {
        let path_str = project_path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid project path"))?;

        debug!(project_path = %path_str, topic_id = topic_id, "get_or_create: looking up instance");

        // Check if instance already exists in memory
        if let Some(instance) = self.get_instance_by_path(project_path).await {
            let inst = instance.lock().await;
            match inst.state().await {
                InstanceState::Running | InstanceState::Starting => {
                    debug!(project_path = %path_str, "Returning existing running instance");
                    drop(inst);
                    self.record_activity(path_str).await;
                    return Ok(instance);
                }
                InstanceState::Stopped | InstanceState::Error => {
                    debug!(project_path = %path_str, "Instance stopped/error, attempting restart");
                    drop(inst);
                    // Try to restart
                    return self.restart_instance_by_path(project_path).await;
                }
                InstanceState::Stopping => {
                    return Err(anyhow!("Instance is currently stopping"));
                }
            }
        }

        // Check database for persisted instance
        let store = self.store.lock().await;
        if let Some(info) = store.get_instance_by_path(path_str).await? {
            debug!(project_path = %path_str, instance_id = %info.id, state = ?info.state, "Found instance in database but not memory");
            // Instance exists in DB but not in memory - spawn new (containers don't survive)
            drop(store);
            return self.spawn_new_instance(project_path, topic_id).await;
        } else {
            debug!(project_path = %path_str, "No instance found in memory or database");
            drop(store);
        }

        // Check max instances limit
        let instances = self.instances.lock().await;
        debug!(
            current_count = instances.len(),
            max = self.config.opencode_max_instances,
            "Checking instance limit"
        );
        if instances.len() >= self.config.opencode_max_instances {
            return Err(anyhow!(
                "Maximum instances limit reached ({})",
                self.config.opencode_max_instances
            ));
        }
        drop(instances);

        // Create new instance
        debug!(project_path = %path_str, "Spawning new instance");
        self.spawn_new_instance(project_path, topic_id).await
    }

    /// Get an instance by ID.
    #[allow(dead_code)]
    // Used by future: instance lookup feature
    pub async fn get_instance(&self, id: &str) -> Option<Arc<Mutex<OpenCodeInstance>>> {
        let instances = self.instances.lock().await;
        instances.get(id).cloned()
    }

    /// Get an instance by project path.
    pub async fn get_instance_by_path(&self, path: &Path) -> Option<Arc<Mutex<OpenCodeInstance>>> {
        let path_str = path.to_str()?;
        let instances = self.instances.lock().await;
        for instance in instances.values() {
            let inst = instance.lock().await;
            if inst.project_path() == path_str {
                drop(inst);
                return Some(instance.clone());
            }
        }
        None
    }

    /// Stop a specific instance by ID.
    pub async fn stop_instance(&self, id: &str) -> Result<()> {
        debug!(instance_id = %id, "Stopping instance");
        let instance = {
            let instances = self.instances.lock().await;
            instances.get(id).cloned()
        };

        if let Some(instance) = instance {
            let inst = instance.lock().await;
            let port = inst.port();
            inst.stop().await?;
            drop(inst);

            debug!(instance_id = %id, port = port, "Instance stopped, releasing port");

            // Release port back to pool
            self.port_pool.release(port).await;

            let store = self.store.lock().await;
            store.update_state(id, InstanceState::Stopped).await?;
            store.update_container_id(id, None).await?;

            let mut instances = self.instances.lock().await;
            instances.remove(id);
            debug!(instance_id = %id, "Instance removed from tracking");

            let mut restart_trackers = self.restart_trackers.lock().await;
            restart_trackers.remove(id);
            let mut activity_trackers = self.activity_trackers.lock().await;
            activity_trackers.remove(id);

            Ok(())
        } else {
            Err(anyhow!("Instance not found: {}", id))
        }
    }

    /// Stop all instances gracefully.
    pub async fn stop_all(&self) -> Result<()> {
        // Signal shutdown to background tasks
        {
            let mut shutdown = self.shutdown_signal.lock().await;
            *shutdown = true;
        }

        let instance_ids: Vec<String> = {
            let instances = self.instances.lock().await;
            instances.keys().cloned().collect()
        };

        debug!(count = instance_ids.len(), "Stopping all instances");

        let mut errors = Vec::new();
        for id in instance_ids {
            if let Err(e) = self.stop_instance(&id).await {
                errors.push(format!("{}: {}", id, e));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to stop some instances: {}",
                errors.join(", ")
            ))
        }
    }

    /// Get manager status statistics.
    pub async fn get_status(&self) -> ManagerStatus {
        let instances = self.instances.lock().await;
        let mut running = 0;
        let mut stopped = 0;
        let mut error = 0;

        for instance in instances.values() {
            let inst = instance.lock().await;
            match inst.state().await {
                InstanceState::Running | InstanceState::Starting => running += 1,
                InstanceState::Stopped | InstanceState::Stopping => stopped += 1,
                InstanceState::Error => error += 1,
            }
        }

        let total_ports = self.config.opencode_port_pool_size as usize;
        let allocated_ports = self.port_pool.allocated_count();

        ManagerStatus {
            total_instances: instances.len(),
            running_instances: running,
            stopped_instances: stopped,
            error_instances: error,
            available_ports: total_ports.saturating_sub(allocated_ports),
        }
    }

    /// Recover instances from database after restart.
    ///
    /// Marks all running instances as stopped (containers don't survive bot restart).
    pub async fn recover_from_db(&self) -> Result<()> {
        debug!("Starting instance recovery from database");
        let store = self.store.lock().await;
        let all_instances = store.get_all_instances().await?;

        debug!(
            instance_count = all_instances.len(),
            "Found instances in database for recovery"
        );

        for info in all_instances {
            debug!(instance_id = %info.id, state = ?info.state, "Evaluating instance for recovery");
            if info.state == InstanceState::Running || info.state == InstanceState::Starting {
                // Mark as stopped (containers don't survive bot restart)
                debug!(instance_id = %info.id, "Marking previously running instance as stopped");
                let _ = store.update_state(&info.id, InstanceState::Stopped).await;
            }
        }

        debug!("Instance recovery complete");
        Ok(())
    }

    pub async fn reconcile_containers(&self) -> Result<()> {
        debug!("Starting container reconciliation");

        let containers = self.runtime.list_containers_by_prefix("oc-").await?;
        let store = self.store.lock().await;
        let instances = store.get_all_instances().await?;
        drop(store);

        let mut container_ids = std::collections::HashSet::new();
        for container in &containers {
            container_ids.insert(container.id.clone());
        }

        let mut db_container_ids = std::collections::HashSet::new();
        for info in &instances {
            if let Some(container_id) = info.container_id.as_ref() {
                db_container_ids.insert(container_id.clone());
                if !container_ids.contains(container_id) {
                    tracing::warn!(
                        instance_id = %info.id,
                        container_id = %container_id,
                        "Container missing for instance, marking error"
                    );
                    let store = self.store.lock().await;
                    store.update_state(&info.id, InstanceState::Error).await?;
                }
            }
        }

        for container in containers {
            if !db_container_ids.contains(&container.id) {
                tracing::info!(
                    container_id = %container.id,
                    name = %container.name,
                    "Orphan container found, stopping and removing"
                );
                self.runtime.stop_container(&container.id, 5).await?;
                self.runtime.remove_container(&container.id, true).await?;
            }
        }

        Ok(())
    }

    /// Start periodic health check monitoring.
    ///
    /// Spawns a background task that checks instance health and handles:
    /// - Crashed instances (auto-restart with backoff)
    /// - Idle instances (stop after timeout)
    pub fn start_health_check_loop(&self) -> tokio::task::JoinHandle<()> {
        let instances = self.instances.clone();
        let restart_trackers = self.restart_trackers.clone();
        let activity_trackers = self.activity_trackers.clone();
        let store = self.store.clone();
        let port_pool = self.port_pool.clone();
        let config = self.config.clone();
        let runtime = self.runtime.clone();
        let shutdown_signal = self.shutdown_signal.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.opencode_health_check_interval);
            debug!(
                interval_ms = config.opencode_health_check_interval.as_millis() as u64,
                "Health check loop started"
            );

            loop {
                interval.tick().await;

                // Check shutdown signal
                {
                    let shutdown = shutdown_signal.lock().await;
                    if *shutdown {
                        break;
                    }
                }

                // Get all instance IDs
                let instance_ids: Vec<String> = {
                    let instances = instances.lock().await;
                    instances.keys().cloned().collect()
                };

                debug!(instance_count = instance_ids.len(), "Health check tick");

                for id in instance_ids {
                    let instance = {
                        let instances = instances.lock().await;
                        instances.get(&id).cloned()
                    };

                    if let Some(instance) = instance {
                        let inst = instance.lock().await;
                        let state = inst.state().await;

                        // Only check running instances
                        if state != InstanceState::Running {
                            continue;
                        }

                        // Check for crash
                        match inst.check_for_crash().await {
                            Ok(true) => {
                                drop(inst);
                                tracing::warn!("Instance {} crashed, attempting restart", id);

                                // Attempt restart with backoff
                                let mut trackers = restart_trackers.lock().await;
                                let tracker = trackers.entry(id.clone()).or_default();

                                if tracker.attempt < MAX_RESTART_ATTEMPTS {
                                    let delay = INITIAL_RESTART_DELAY
                                        .mul_f64(2_f64.powi(tracker.attempt as i32));
                                    tracker.attempt += 1;
                                    tracker.last_attempt = Some(Instant::now());
                                    drop(trackers);

                                    tracing::info!(
                                        "Waiting {:?} before restart attempt for {}",
                                        delay,
                                        id
                                    );
                                    tokio::time::sleep(delay).await;

                                    let (project_path, topic_id) = {
                                        let store_guard = store.lock().await;
                                        match store_guard.get_instance(&id).await {
                                            Ok(Some(info)) => {
                                                (info.project_path.clone(), info.topic_id)
                                            }
                                            _ => {
                                                tracing::error!(
                                                    "Failed to get instance info for restart of {}",
                                                    id
                                                );
                                                let _ = store_guard
                                                    .update_state(&id, InstanceState::Error)
                                                    .await;
                                                continue;
                                            }
                                        }
                                    };

                                    let old_port = {
                                        let inst = instance.lock().await;
                                        inst.port()
                                    };

                                    {
                                        let mut instances_lock = instances.lock().await;
                                        instances_lock.remove(&id);
                                    }

                                    port_pool.release(old_port).await;
                                    let new_port = match port_pool.allocate().await {
                                        Ok(p) => p,
                                        Err(e) => {
                                            tracing::error!(
                                                "No ports available for restart of {}: {}",
                                                id,
                                                e
                                            );
                                            let store_guard = store.lock().await;
                                            let _ = store_guard
                                                .update_state(&id, InstanceState::Error)
                                                .await;
                                            continue;
                                        }
                                    };

                                    let new_id = format!(
                                        "inst_{}",
                                        uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
                                    );
                                    let instance_config = InstanceConfig {
                                        id: new_id.clone(),
                                        project_path: project_path.clone(),
                                        port: new_port,
                                        auto_start: true,
                                        opencode_path: config
                                            .opencode_path
                                            .to_string_lossy()
                                            .to_string(),
                                    };

                                    let container_config = ContainerConfig {
                                        instance_id: new_id.clone(),
                                        image: config.docker_image.clone(),
                                        host_port: new_port,
                                        container_port: config.container_port,
                                        worktree_path: project_path.clone(),
                                        config_mount_path: config
                                            .opencode_config_path
                                            .to_string_lossy()
                                            .to_string(),
                                        opencode_data_path: config
                                            .opencode_data_path
                                            .to_string_lossy()
                                            .to_string(),
                                        topic_id,
                                        env_vars: config.env_passthrough.clone(),
                                    };

                                    let spawn_result = OpenCodeInstance::spawn(
                                        instance_config,
                                        new_port,
                                        runtime.clone(),
                                        container_config,
                                    )
                                    .await;
                                    match spawn_result {
                                        Ok((new_instance, container_id)) => {
                                            let new_instance = Arc::new(Mutex::new(new_instance));

                                            let ready = {
                                                let inst = new_instance.lock().await;
                                                inst.wait_for_ready(
                                                    config.opencode_startup_timeout,
                                                    Duration::from_millis(500),
                                                )
                                                .await
                                            };

                                            match ready {
                                                Ok(true) => {
                                                    let info = InstanceInfo {
                                                        id: new_id.clone(),
                                                        state: InstanceState::Running,
                                                        project_path: project_path.clone(),
                                                        port: new_port,
                                                        pid: None,
                                                        container_id: Some(container_id.clone()),
                                                        started_at: Some(
                                                            std::time::SystemTime::now()
                                                                .duration_since(
                                                                    std::time::UNIX_EPOCH,
                                                                )
                                                                .unwrap()
                                                                .as_secs()
                                                                as i64,
                                                        ),
                                                        stopped_at: None,
                                                        topic_id,
                                                    };

                                                    {
                                                        let store_guard = store.lock().await;
                                                        if let Err(e) = store_guard
                                                            .save_instance(&info, None)
                                                            .await
                                                        {
                                                            tracing::error!(
                                                                "Failed to save restarted instance: {:?}",
                                                                e
                                                            );
                                                            port_pool.release(new_port).await;
                                                            continue;
                                                        }
                                                        let _ = store_guard
                                                            .update_container_id(
                                                                &new_id,
                                                                Some(&container_id),
                                                            )
                                                            .await;
                                                        let _ = store_guard
                                                            .update_state(&id, InstanceState::Error)
                                                            .await;
                                                    }

                                                    instances
                                                        .lock()
                                                        .await
                                                        .insert(new_id.clone(), new_instance);

                                                    // Carry over attempt count so MAX_RESTART_ATTEMPTS spans the full lineage
                                                    {
                                                        let mut trk = restart_trackers.lock().await;
                                                        if let Some(old_tracker) = trk.remove(&id) {
                                                            trk.insert(new_id.clone(), old_tracker);
                                                        }
                                                    }

                                                    {
                                                        let mut at = activity_trackers.lock().await;
                                                        at.remove(&id);
                                                        at.insert(
                                                            new_id.clone(),
                                                            ActivityTracker::default(),
                                                        );
                                                    }

                                                    tracing::info!(
                                                        "Successfully restarted instance {} as {}",
                                                        id,
                                                        new_id
                                                    );
                                                }
                                                _ => {
                                                    tracing::error!(
                                                        "Restarted instance {} failed readiness check",
                                                        id
                                                    );
                                                    let inst = new_instance.lock().await;
                                                    let _ = inst.stop().await;
                                                    drop(inst);
                                                    port_pool.release(new_port).await;
                                                    let store_guard = store.lock().await;
                                                    let _ = store_guard
                                                        .update_state(&id, InstanceState::Error)
                                                        .await;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to spawn restart for {}: {:?}",
                                                id,
                                                e
                                            );
                                            port_pool.release(new_port).await;
                                            let store_guard = store.lock().await;
                                            let _ = store_guard
                                                .update_state(&id, InstanceState::Error)
                                                .await;
                                        }
                                    }
                                } else {
                                    drop(trackers);
                                    tracing::error!(
                                        "Instance {} exceeded max restart attempts, marking as error",
                                        id
                                    );
                                    let store_guard = store.lock().await;
                                    let _ =
                                        store_guard.update_state(&id, InstanceState::Error).await;
                                }
                            }
                            Ok(false) => {
                                // Instance is healthy, reset restart tracker
                                debug!(instance_id = %id, "Instance healthy, restart tracker reset");
                                let mut trackers = restart_trackers.lock().await;
                                trackers.remove(&id);
                                drop(inst);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to check instance {} health: {}", id, e);
                                drop(inst);
                            }
                        }

                        // Check idle timeout
                        let activity = {
                            let activity_trackers = activity_trackers.lock().await;
                            activity_trackers.get(&id).cloned()
                        };

                        if let Some(activity) = activity {
                            if activity.last_activity.elapsed() > config.opencode_idle_timeout {
                                debug!(instance_id = %id, idle_secs = activity.last_activity.elapsed().as_secs(), timeout_secs = config.opencode_idle_timeout.as_secs(), "Idle timeout check");
                                tracing::info!("Instance {} idle timeout reached, stopping", id);
                                let instance = {
                                    let instances = instances.lock().await;
                                    instances.get(&id).cloned()
                                };

                                if let Some(instance) = instance {
                                    let inst = instance.lock().await;
                                    let port = inst.port();
                                    let _ = inst.stop().await;
                                    drop(inst);

                                    port_pool.release(port).await;

                                    let store = store.lock().await;
                                    let _ = store.update_state(&id, InstanceState::Stopped).await;

                                    let mut instances = instances.lock().await;
                                    instances.remove(&id);

                                    let mut activity_trackers = activity_trackers.lock().await;
                                    activity_trackers.remove(&id);

                                    let mut restart_trackers = restart_trackers.lock().await;
                                    restart_trackers.remove(&id);
                                }
                            }
                        }
                    }
                }
            }
        })
    }

    /// Record activity for an instance (for idle timeout tracking).
    pub async fn record_activity(&self, id: &str) {
        debug!(instance_id = %id, "Activity recorded for instance");
        let mut activity_trackers = self.activity_trackers.lock().await;
        let tracker = activity_trackers.entry(id.to_string()).or_default();
        tracker.last_activity = Instant::now();
    }

    /// Spawn a new OpenCode instance.
    async fn spawn_new_instance(
        &self,
        project_path: &Path,
        topic_id: i32,
    ) -> Result<Arc<Mutex<OpenCodeInstance>>> {
        let path_str = project_path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid project path"))?;

        // Allocate port
        let port = self.port_pool.allocate().await?;
        debug!(port = port, project_path = %path_str, "Port allocated for new instance");

        // Generate unique ID
        let id = format!(
            "inst_{}",
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
        );
        debug!(instance_id = %id, port = port, "Spawning OpenCode instance");

        // Create config
        let instance_config = InstanceConfig {
            id: id.clone(),
            project_path: path_str.to_string(),
            port,
            auto_start: true,
            opencode_path: self.config.opencode_path.to_string_lossy().to_string(),
        };

        let container_config = ContainerConfig {
            instance_id: id.clone(),
            image: self.config.docker_image.clone(),
            host_port: port,
            container_port: self.config.container_port,
            worktree_path: path_str.to_string(),
            config_mount_path: self
                .config
                .opencode_config_path
                .to_string_lossy()
                .to_string(),
            opencode_data_path: self.config.opencode_data_path.to_string_lossy().to_string(),
            topic_id,
            env_vars: self.config.env_passthrough.clone(),
        };

        // Spawn instance
        let spawn_result = OpenCodeInstance::spawn(
            instance_config.clone(),
            port,
            self.runtime.clone(),
            container_config,
        )
        .await;
        let (instance, container_id) = match spawn_result {
            Ok(inst) => inst,
            Err(e) => {
                // Release port on failure
                self.port_pool.release(port).await;
                return Err(e);
            }
        };

        debug!(instance_id = %id, "Process spawned, waiting for readiness");

        let instance = Arc::new(Mutex::new(instance));

        // Wait for instance to be ready
        {
            let inst = instance.lock().await;
            let ready = inst
                .wait_for_ready(
                    self.config.opencode_startup_timeout,
                    Duration::from_millis(500),
                )
                .await?;

            debug!(instance_id = %id, ready = ready, "Readiness check result");

            if !ready {
                inst.stop().await?;
                drop(inst);
                self.port_pool.release(port).await;
                return Err(anyhow!("Instance failed to start within timeout"));
            }
        }

        // Save to database
        let info = InstanceInfo {
            id: id.clone(),
            state: InstanceState::Running,
            project_path: path_str.to_string(),
            port,
            pid: None,
            container_id: Some(container_id.clone()),
            started_at: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
            ),
            stopped_at: None,
            topic_id,
        };

        let store = self.store.lock().await;
        store.save_instance(&info, None).await?;
        store.update_container_id(&id, Some(&container_id)).await?;
        drop(store);
        debug!(instance_id = %id, "Instance saved to database");

        // Add to instances map
        let mut instances = self.instances.lock().await;
        instances.insert(id.clone(), instance.clone());
        debug!(instance_id = %id, "Instance added to active map");

        // Initialize activity tracker
        let mut activity_trackers = self.activity_trackers.lock().await;
        activity_trackers.insert(id.clone(), ActivityTracker::default());

        Ok(instance)
    }

    /// Restart an instance by project path.
    async fn restart_instance_by_path(
        &self,
        project_path: &Path,
    ) -> Result<Arc<Mutex<OpenCodeInstance>>> {
        let path_str = project_path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid project path"))?;

        debug!(project_path = %path_str, "Restart requested");

        // Get existing instance
        let (id, old_port) = {
            if let Some(instance) = self.get_instance_by_path(project_path).await {
                let inst = instance.lock().await;
                (inst.id().to_string(), inst.port())
            } else {
                return Err(anyhow!("Instance not found for path: {}", path_str));
            }
        };

        debug!(instance_id = %id, old_port = old_port, "Found instance to restart");

        // Get topic_id from store
        let topic_id = {
            let store = self.store.lock().await;
            match store.get_instance(&id).await {
                Ok(Some(info)) => info.topic_id,
                _ => {
                    return Err(anyhow!("Failed to get instance info for restart of {}", id));
                }
            }
        };

        // Check restart tracker
        let mut trackers = self.restart_trackers.lock().await;
        let tracker = trackers.entry(id.clone()).or_default();

        if tracker.attempt >= MAX_RESTART_ATTEMPTS {
            return Err(anyhow!(
                "Instance {} has exceeded maximum restart attempts",
                id
            ));
        }

        let delay = INITIAL_RESTART_DELAY.mul_f64(2_f64.powi(tracker.attempt as i32));
        let attempt = tracker.attempt;
        tracker.attempt += 1;
        tracker.last_attempt = Some(Instant::now());
        drop(trackers);

        debug!(instance_id = %id, attempt = attempt, delay_ms = delay.as_millis() as u64, "Restart backoff");

        // Wait for backoff delay
        tokio::time::sleep(delay).await;

        {
            let mut instances = self.instances.lock().await;
            instances.remove(&id);
        }

        self.port_pool.release(old_port).await;

        self.spawn_new_instance(project_path, topic_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::container::mock::{MockAction, MockRuntime};
    use crate::orchestrator::container::{ContainerInfo, ContainerState};
    use tempfile::TempDir;

    async fn create_test_manager() -> (InstanceManager, TempDir, Arc<MockRuntime>) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create minimal config
        let config = Config {
            telegram_bot_token: "test".to_string(),
            telegram_chat_ids: vec![-1001234567890],
            telegram_allowed_users: vec![],
            handle_general_topic: true,
            opencode_path: std::path::PathBuf::from("/nonexistent/opencode-test-binary"),
            opencode_max_instances: 5,
            opencode_idle_timeout: Duration::from_secs(300),
            opencode_port_start: 14100,
            opencode_port_pool_size: 10,
            opencode_health_check_interval: Duration::from_secs(30),
            opencode_startup_timeout: Duration::from_secs(5),
            opencode_data_path: std::path::PathBuf::from("/tmp/opencode-data"),
            orchestrator_db_path: db_path.clone(),
            topic_db_path: temp_dir.path().join("topics.db"),
            log_db_path: temp_dir.path().join("logs.db"),
            project_base_path: temp_dir.path().to_path_buf(),
            auto_create_project_dirs: true,
            docker_image: "ghcr.io/sst/opencode".to_string(),
            opencode_config_path: std::path::PathBuf::from("/tmp/oc-config"),
            container_port: 8080,
            env_passthrough: vec![],
        };

        let store = OrchestratorStore::new(&db_path).await.unwrap();
        let port_pool = PortPool::new(14100, 10);
        let runtime = Arc::new(MockRuntime::new());

        let manager = InstanceManager::new(Arc::new(config), store, port_pool, runtime.clone())
            .await
            .unwrap();

        (manager, temp_dir, runtime)
    }

    #[tokio::test]
    async fn test_new_creates_manager() {
        let (manager, _temp_dir, _runtime) = create_test_manager().await;
        assert_eq!(manager.config.opencode_max_instances, 5);
        assert_eq!(manager.config.opencode_port_start, 14100);
    }

    #[tokio::test]
    async fn test_get_instance_returns_none_when_not_found() {
        let (manager, _temp_dir, _runtime) = create_test_manager().await;
        let result = manager.get_instance("nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_instance_by_path_returns_none_when_not_found() {
        let (manager, _temp_dir, _runtime) = create_test_manager().await;
        let result = manager
            .get_instance_by_path(Path::new("/nonexistent/path"))
            .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_status_initial_empty() {
        let (manager, _temp_dir, _runtime) = create_test_manager().await;
        let status = manager.get_status().await;

        assert_eq!(status.total_instances, 0);
        assert_eq!(status.running_instances, 0);
        assert_eq!(status.stopped_instances, 0);
        assert_eq!(status.error_instances, 0);
        assert_eq!(status.available_ports, 10);
    }

    #[tokio::test]
    async fn test_stop_instance_returns_error_when_not_found() {
        let (manager, _temp_dir, _runtime) = create_test_manager().await;
        let result = manager.stop_instance("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_stop_all_succeeds_when_empty() {
        let (manager, _temp_dir, _runtime) = create_test_manager().await;
        let result = manager.stop_all().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_recover_from_db_succeeds_when_empty() {
        let (manager, _temp_dir, _runtime) = create_test_manager().await;
        let result = manager.recover_from_db().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_reconcile_orphan_containers_removed() {
        let (manager, _temp_dir, runtime) = create_test_manager().await;

        let orphan = ContainerInfo {
            id: "orphan-1".to_string(),
            name: "oc-orphan-1".to_string(),
            state: ContainerState::Running,
        };

        *runtime.list_result.lock().unwrap() = Ok(vec![orphan]);

        manager.reconcile_containers().await.unwrap();

        let actions = runtime.recorded_actions();
        assert!(matches!(actions[0], MockAction::ListContainers { .. }));
        assert!(matches!(actions[1], MockAction::StopContainer { ref id, .. } if id == "orphan-1"));
        assert!(
            matches!(actions[2], MockAction::RemoveContainer { ref id, .. } if id == "orphan-1")
        );
    }

    #[tokio::test]
    async fn test_reconcile_db_without_container_marked_error() {
        let (manager, _temp_dir, runtime) = create_test_manager().await;

        *runtime.list_result.lock().unwrap() = Ok(vec![]);

        let info = InstanceInfo {
            id: "inst-missing".to_string(),
            state: InstanceState::Running,
            project_path: "/tmp/project".to_string(),
            port: 14101,
            pid: None,
            container_id: Some("missing-container".to_string()),
            started_at: None,
            stopped_at: None,
            topic_id: 999,
        };

        {
            let store = manager.store.lock().await;
            store.save_instance(&info, None).await.unwrap();
        }

        manager.reconcile_containers().await.unwrap();

        let store = manager.store.lock().await;
        let updated = store.get_instance("inst-missing").await.unwrap().unwrap();
        assert_eq!(updated.state, InstanceState::Error);
    }

    #[tokio::test]
    async fn test_reconcile_matching_containers_kept() {
        let (manager, _temp_dir, runtime) = create_test_manager().await;

        let container = ContainerInfo {
            id: "match-1".to_string(),
            name: "oc-match-1".to_string(),
            state: ContainerState::Running,
        };

        *runtime.list_result.lock().unwrap() = Ok(vec![container]);

        let info = InstanceInfo {
            id: "inst-match".to_string(),
            state: InstanceState::Running,
            project_path: "/tmp/project".to_string(),
            port: 14102,
            pid: None,
            container_id: Some("match-1".to_string()),
            started_at: None,
            stopped_at: None,
            topic_id: 888,
        };

        {
            let store = manager.store.lock().await;
            store.save_instance(&info, None).await.unwrap();
        }

        manager.reconcile_containers().await.unwrap();

        let actions = runtime.recorded_actions();
        assert!(actions
            .iter()
            .all(|action| !matches!(action, MockAction::StopContainer { .. })));
        assert!(actions
            .iter()
            .all(|action| !matches!(action, MockAction::RemoveContainer { .. })));

        let store = manager.store.lock().await;
        let updated = store.get_instance("inst-match").await.unwrap().unwrap();
        assert_eq!(updated.state, InstanceState::Running);
    }

    #[tokio::test]
    async fn test_record_activity_creates_tracker() {
        let (manager, _temp_dir, _runtime) = create_test_manager().await;
        manager.record_activity("test-instance").await;

        let activity_trackers = manager.activity_trackers.lock().await;
        assert!(activity_trackers.contains_key("test-instance"));
    }

    #[tokio::test]
    async fn test_record_activity_updates_timestamp() {
        let (manager, _temp_dir, _runtime) = create_test_manager().await;
        manager.record_activity("test-instance").await;

        // Wait a bit
        tokio::time::sleep(Duration::from_millis(10)).await;

        let first_activity = {
            let trackers = manager.activity_trackers.lock().await;
            trackers.get("test-instance").unwrap().last_activity
        };

        manager.record_activity("test-instance").await;

        let second_activity = {
            let trackers = manager.activity_trackers.lock().await;
            trackers.get("test-instance").unwrap().last_activity
        };

        assert!(second_activity > first_activity);
    }

    #[tokio::test]
    async fn test_manager_status_struct() {
        let status = ManagerStatus {
            total_instances: 10,
            running_instances: 5,
            stopped_instances: 3,
            error_instances: 2,
            available_ports: 90,
        };

        assert_eq!(status.total_instances, 10);
        assert_eq!(status.running_instances, 5);
        assert_eq!(status.stopped_instances, 3);
        assert_eq!(status.error_instances, 2);
        assert_eq!(status.available_ports, 90);
    }

    #[tokio::test]
    async fn test_restart_tracker_default() {
        let tracker = RestartTracker::default();
        assert_eq!(tracker.attempt, 0);
        assert!(tracker.last_attempt.is_none());
    }

    #[tokio::test]
    async fn test_activity_tracker_default() {
        let tracker = ActivityTracker::default();
        // Should be very recent
        assert!(tracker.last_activity.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_get_or_create_enforces_max_instances() {
        use crate::orchestrator::container::ContainerConfig;
        use crate::orchestrator::instance::OpenCodeInstance;
        use crate::types::instance::InstanceConfig;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = Config {
            telegram_bot_token: "test".to_string(),
            telegram_chat_ids: vec![-1001234567890],
            telegram_allowed_users: vec![],
            handle_general_topic: true,
            opencode_path: std::path::PathBuf::from("opencode"),
            opencode_max_instances: 1,
            opencode_idle_timeout: Duration::from_secs(300),
            opencode_port_start: 14200,
            opencode_port_pool_size: 10,
            opencode_health_check_interval: Duration::from_secs(30),
            opencode_startup_timeout: Duration::from_secs(1),
            opencode_data_path: std::path::PathBuf::from("/tmp/opencode-data"),
            orchestrator_db_path: db_path.clone(),
            topic_db_path: temp_dir.path().join("topics.db"),
            log_db_path: temp_dir.path().join("logs.db"),
            project_base_path: temp_dir.path().to_path_buf(),
            auto_create_project_dirs: true,
            docker_image: "ghcr.io/sst/opencode".to_string(),
            opencode_config_path: std::path::PathBuf::from("/tmp/oc-config"),
            container_port: 8080,
            env_passthrough: vec![],
        };

        let store = OrchestratorStore::new(&db_path).await.unwrap();
        let port_pool = PortPool::new(14200, 10);
        let runtime = Arc::new(MockRuntime::new());

        let manager = InstanceManager::new(Arc::new(config), store, port_pool, runtime.clone())
            .await
            .unwrap();

        let inst_config = InstanceConfig {
            id: "inst_test".to_string(),
            project_path: "/test/existing".to_string(),
            port: 14200,
            auto_start: true,
            opencode_path: "opencode".to_string(),
        };
        let container_config = ContainerConfig {
            instance_id: "inst_test".to_string(),
            image: "ghcr.io/sst/opencode".to_string(),
            host_port: 14200,
            container_port: 8080,
            worktree_path: "/test/existing".to_string(),
            config_mount_path: "/tmp/oc-config".to_string(),
            opencode_data_path: "/tmp/opencode-data".to_string(),
            topic_id: 100,
            env_vars: vec![],
        };
        let (instance, _container_id) =
            OpenCodeInstance::spawn(inst_config, 14200, runtime, container_config)
                .await
                .unwrap();
        {
            let mut instances = manager.instances.lock().await;
            instances.insert(
                "inst_test".to_string(),
                Arc::new(tokio::sync::Mutex::new(instance)),
            );
        }

        // Second creation should fail with max instances limit
        let result = manager.get_or_create(Path::new("/test/another"), 999).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Maximum instances limit"));
    }

    #[tokio::test]
    async fn test_concurrent_access_to_manager() {
        let (manager, _temp_dir, _runtime) = create_test_manager().await;
        let manager = Arc::new(manager);

        // Spawn multiple concurrent tasks
        let mut handles = vec![];
        for i in 0..5 {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move {
                // Record activity from multiple tasks
                manager_clone
                    .record_activity(&format!("instance-{}", i))
                    .await;
                manager_clone.get_status().await
            });
            handles.push(handle);
        }

        for handle in handles {
            let status = handle.await.unwrap();
            // Status should be consistent
            assert_eq!(status.total_instances, 0);
        }

        // All activity trackers should be present
        let trackers = manager.activity_trackers.lock().await;
        assert_eq!(trackers.len(), 5);
    }

    #[tokio::test]
    async fn test_health_check_loop_can_be_stopped() {
        let (manager, _temp_dir, _runtime) = create_test_manager().await;

        // Start health check loop
        let handle = manager.start_health_check_loop();

        // Signal shutdown
        {
            let mut shutdown = manager.shutdown_signal.lock().await;
            *shutdown = true;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;

        handle.abort();
    }

    #[tokio::test]
    async fn test_port_allocation_on_spawn_failure() {
        let (manager, temp_dir, runtime) = create_test_manager().await;

        // Port pool should have all ports available initially
        let initial_count = manager.port_pool.allocated_count();
        assert_eq!(initial_count, 0);

        // Try to spawn (will fail because opencode is not installed)
        let project_path = temp_dir.path().join("test-project");
        std::fs::create_dir_all(&project_path).unwrap();

        {
            let mut create_result = runtime.create_result.lock().unwrap();
            *create_result = Err("image not found".to_string());
        }

        let result = manager.get_or_create(&project_path, 777).await;

        // Should fail because container creation is forced to error
        assert!(result.is_err());

        // Port should be released on failure
        let final_count = manager.port_pool.allocated_count();
        assert_eq!(final_count, 0);
    }
}
