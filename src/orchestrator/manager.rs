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
use crate::orchestrator::instance::OpenCodeInstance;
use crate::orchestrator::port_pool::PortPool;
use crate::orchestrator::store::OrchestratorStore;
use crate::types::instance::{InstanceConfig, InstanceInfo, InstanceState, InstanceType};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

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
    ) -> Result<Self> {
        Ok(Self {
            config,
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
    pub async fn get_or_create(&self, project_path: &Path) -> Result<Arc<Mutex<OpenCodeInstance>>> {
        let path_str = project_path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid project path"))?;

        // Check if instance already exists in memory
        if let Some(instance) = self.get_instance_by_path(project_path).await {
            let inst = instance.lock().await;
            match inst.state().await {
                InstanceState::Running | InstanceState::Starting => {
                    drop(inst);
                    self.record_activity(path_str).await;
                    return Ok(instance);
                }
                InstanceState::Stopped | InstanceState::Error => {
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
            drop(store);
            // Instance exists in DB but not in memory, try to recover
            if info.state == InstanceState::Running || info.state == InstanceState::Starting {
                // Try to restore from DB
                return self.restore_instance(&info).await;
            }
        } else {
            drop(store);
        }

        // Check max instances limit
        let instances = self.instances.lock().await;
        if instances.len() >= self.config.opencode_max_instances {
            return Err(anyhow!(
                "Maximum instances limit reached ({})",
                self.config.opencode_max_instances
            ));
        }
        drop(instances);

        // Create new instance
        self.spawn_new_instance(project_path).await
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
        let instance = {
            let instances = self.instances.lock().await;
            instances.get(id).cloned()
        };

        if let Some(instance) = instance {
            let inst = instance.lock().await;
            let port = inst.port();
            inst.stop().await?;
            drop(inst);

            // Release port back to pool
            self.port_pool.release(port).await;

            let store = self.store.lock().await;
            store.update_state(id, InstanceState::Stopped).await?;

            let mut instances = self.instances.lock().await;
            instances.remove(id);

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
    /// Attempts to restore instances that were running before shutdown.
    pub async fn recover_from_db(&self) -> Result<()> {
        let store = self.store.lock().await;
        let all_instances = store.get_all_instances().await?;
        drop(store);

        for info in all_instances {
            if info.state == InstanceState::Running || info.state == InstanceState::Starting {
                // Try to restore instance
                if let Err(e) = self.restore_instance(&info).await {
                    tracing::warn!(
                        "Failed to restore instance {}: {}. Marking as stopped.",
                        info.id,
                        e
                    );
                    // Mark as stopped in database
                    let store = self.store.lock().await;
                    let _ = store.update_state(&info.id, InstanceState::Stopped).await;
                }
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
        let shutdown_signal = self.shutdown_signal.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.opencode_health_check_interval);

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

                                    let project_path = {
                                        let store_guard = store.lock().await;
                                        match store_guard.get_instance(&id).await {
                                            Ok(Some(info)) => info.project_path.clone(),
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
                                        instance_type: InstanceType::Managed,
                                        project_path: project_path.clone(),
                                        port: new_port,
                                        auto_start: true,
                                    };

                                    match OpenCodeInstance::spawn(instance_config, new_port).await {
                                        Ok(new_instance) => {
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
                                                        instance_type: InstanceType::Managed,
                                                        project_path: project_path.clone(),
                                                        port: new_port,
                                                        pid: None,
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
        let mut activity_trackers = self.activity_trackers.lock().await;
        let tracker = activity_trackers.entry(id.to_string()).or_default();
        tracker.last_activity = Instant::now();
    }

    /// Spawn a new OpenCode instance.
    async fn spawn_new_instance(
        &self,
        project_path: &Path,
    ) -> Result<Arc<Mutex<OpenCodeInstance>>> {
        let path_str = project_path
            .to_str()
            .ok_or_else(|| anyhow!("Invalid project path"))?;

        // Allocate port
        let port = self.port_pool.allocate().await?;

        // Generate unique ID
        let id = format!(
            "inst_{}",
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
        );

        // Create config
        let instance_config = InstanceConfig {
            id: id.clone(),
            instance_type: InstanceType::Managed,
            project_path: path_str.to_string(),
            port,
            auto_start: true,
        };

        // Spawn instance
        let instance = match OpenCodeInstance::spawn(instance_config.clone(), port).await {
            Ok(inst) => inst,
            Err(e) => {
                // Release port on failure
                self.port_pool.release(port).await;
                return Err(e);
            }
        };

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
            instance_type: InstanceType::Managed,
            project_path: path_str.to_string(),
            port,
            pid: None,
            started_at: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
            ),
            stopped_at: None,
        };

        let store = self.store.lock().await;
        store.save_instance(&info, None).await?;
        drop(store);

        // Add to instances map
        let mut instances = self.instances.lock().await;
        instances.insert(id.clone(), instance.clone());

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

        // Get existing instance
        let (id, old_port) = {
            if let Some(instance) = self.get_instance_by_path(project_path).await {
                let inst = instance.lock().await;
                (inst.id().to_string(), inst.port())
            } else {
                return Err(anyhow!("Instance not found for path: {}", path_str));
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
        tracker.attempt += 1;
        tracker.last_attempt = Some(Instant::now());
        drop(trackers);

        // Wait for backoff delay
        tokio::time::sleep(delay).await;

        {
            let mut instances = self.instances.lock().await;
            instances.remove(&id);
        }

        self.port_pool.release(old_port).await;

        self.spawn_new_instance(project_path).await
    }

    /// Restore an instance from database info.
    async fn restore_instance(&self, info: &InstanceInfo) -> Result<Arc<Mutex<OpenCodeInstance>>> {
        let project_path = Path::new(&info.project_path);

        // Check if port is available
        if !self.port_pool.is_available(info.port).await {
            // Port is in use, need to allocate new port
            return self.spawn_new_instance(project_path).await;
        }

        // Try to create external instance (process might still be running)
        let instance_config = InstanceConfig {
            id: info.id.clone(),
            instance_type: info.instance_type.clone(),
            project_path: info.project_path.clone(),
            port: info.port,
            auto_start: true,
        };

        let instance = OpenCodeInstance::external(instance_config, info.port, info.pid)?;

        // Check health
        if instance.health_check().await? {
            // Instance is still running
            let instance = Arc::new(Mutex::new(instance));
            let mut instances = self.instances.lock().await;
            instances.insert(info.id.clone(), instance.clone());

            // Initialize activity tracker
            let mut activity_trackers = self.activity_trackers.lock().await;
            activity_trackers.insert(info.id.clone(), ActivityTracker::default());

            Ok(instance)
        } else {
            // Instance is not running, spawn new
            self.spawn_new_instance(project_path).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_manager() -> (InstanceManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create minimal config
        let config = Config {
            telegram_bot_token: "test".to_string(),
            telegram_chat_id: -1001234567890,
            telegram_allowed_users: vec![],
            handle_general_topic: true,
            opencode_path: std::path::PathBuf::from("opencode"),
            opencode_max_instances: 5,
            opencode_idle_timeout: Duration::from_secs(300),
            opencode_port_start: 14100,
            opencode_port_pool_size: 10,
            opencode_health_check_interval: Duration::from_secs(30),
            opencode_startup_timeout: Duration::from_secs(5),
            orchestrator_db_path: db_path.clone(),
            topic_db_path: temp_dir.path().join("topics.db"),
            log_db_path: temp_dir.path().join("logs.db"),
            project_base_path: temp_dir.path().to_path_buf(),
            auto_create_project_dirs: true,
            api_port: 4200,
            api_key: None,
        };

        let store = OrchestratorStore::new(&db_path).await.unwrap();
        let port_pool = PortPool::new(14100, 10);

        let manager = InstanceManager::new(Arc::new(config), store, port_pool)
            .await
            .unwrap();

        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_new_creates_manager() {
        let (manager, _temp_dir) = create_test_manager().await;
        assert_eq!(manager.config.opencode_max_instances, 5);
        assert_eq!(manager.config.opencode_port_start, 14100);
    }

    #[tokio::test]
    async fn test_get_instance_returns_none_when_not_found() {
        let (manager, _temp_dir) = create_test_manager().await;
        let result = manager.get_instance("nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_instance_by_path_returns_none_when_not_found() {
        let (manager, _temp_dir) = create_test_manager().await;
        let result = manager
            .get_instance_by_path(Path::new("/nonexistent/path"))
            .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_status_initial_empty() {
        let (manager, _temp_dir) = create_test_manager().await;
        let status = manager.get_status().await;

        assert_eq!(status.total_instances, 0);
        assert_eq!(status.running_instances, 0);
        assert_eq!(status.stopped_instances, 0);
        assert_eq!(status.error_instances, 0);
        assert_eq!(status.available_ports, 10);
    }

    #[tokio::test]
    async fn test_stop_instance_returns_error_when_not_found() {
        let (manager, _temp_dir) = create_test_manager().await;
        let result = manager.stop_instance("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_stop_all_succeeds_when_empty() {
        let (manager, _temp_dir) = create_test_manager().await;
        let result = manager.stop_all().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_recover_from_db_succeeds_when_empty() {
        let (manager, _temp_dir) = create_test_manager().await;
        let result = manager.recover_from_db().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_record_activity_creates_tracker() {
        let (manager, _temp_dir) = create_test_manager().await;
        manager.record_activity("test-instance").await;

        let activity_trackers = manager.activity_trackers.lock().await;
        assert!(activity_trackers.contains_key("test-instance"));
    }

    #[tokio::test]
    async fn test_record_activity_updates_timestamp() {
        let (manager, _temp_dir) = create_test_manager().await;
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
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create config with max 1 instance
        let config = Config {
            telegram_bot_token: "test".to_string(),
            telegram_chat_id: -1001234567890,
            telegram_allowed_users: vec![],
            handle_general_topic: true,
            opencode_path: std::path::PathBuf::from("opencode"),
            opencode_max_instances: 1, // Only 1 allowed
            opencode_idle_timeout: Duration::from_secs(300),
            opencode_port_start: 14200,
            opencode_port_pool_size: 10,
            opencode_health_check_interval: Duration::from_secs(30),
            opencode_startup_timeout: Duration::from_secs(1),
            orchestrator_db_path: db_path.clone(),
            topic_db_path: temp_dir.path().join("topics.db"),
            log_db_path: temp_dir.path().join("logs.db"),
            project_base_path: temp_dir.path().to_path_buf(),
            auto_create_project_dirs: true,
            api_port: 4200,
            api_key: None,
        };

        let store = OrchestratorStore::new(&db_path).await.unwrap();
        let port_pool = PortPool::new(14200, 10);

        let manager = InstanceManager::new(Arc::new(config), store, port_pool)
            .await
            .unwrap();

        // Manually add one instance to hit limit
        let instance_config = InstanceConfig {
            id: "existing".to_string(),
            instance_type: InstanceType::Managed,
            project_path: "/test/existing".to_string(),
            port: 14200,
            auto_start: true,
        };
        let instance = OpenCodeInstance::external(instance_config, 14200, None).unwrap();
        {
            let mut instances = manager.instances.lock().await;
            instances.insert("existing".to_string(), Arc::new(Mutex::new(instance)));
        }

        // Try to create another should fail
        let result = manager.get_or_create(Path::new("/test/another")).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Maximum instances limit"));
    }

    #[tokio::test]
    async fn test_concurrent_access_to_manager() {
        let (manager, _temp_dir) = create_test_manager().await;
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
        let (manager, _temp_dir) = create_test_manager().await;

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
        let (manager, temp_dir) = create_test_manager().await;

        // Port pool should have all ports available initially
        let initial_count = manager.port_pool.allocated_count();
        assert_eq!(initial_count, 0);

        // Try to spawn (will fail because opencode is not installed)
        let project_path = temp_dir.path().join("test-project");
        std::fs::create_dir_all(&project_path).unwrap();

        let result = manager.get_or_create(&project_path).await;

        // Should fail because opencode binary doesn't exist
        assert!(result.is_err());

        // Port should be released on failure
        let final_count = manager.port_pool.allocated_count();
        assert_eq!(final_count, 0);
    }
}
