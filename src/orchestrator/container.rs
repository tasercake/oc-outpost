use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use tracing::debug;

#[derive(Debug, Clone, PartialEq)]
pub enum ContainerState {
    Running,
    Exited(i64),
    Created,
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub state: ContainerState,
}

#[derive(Debug, Clone)]
pub struct ContainerConfig {
    pub instance_id: String,
    pub image: String,
    pub host_port: u16,
    pub container_port: u16,
    pub worktree_path: String,
    pub config_mount_path: String,
    pub env_vars: Vec<String>,
}

impl ContainerConfig {
    pub fn container_name(&self) -> String {
        format!("oc-{}", self.instance_id)
    }

    pub fn cmd(&self) -> Vec<String> {
        vec![
            "opencode".to_string(),
            "serve".to_string(),
            "--port".to_string(),
            self.container_port.to_string(),
            "--project".to_string(),
            "/workspace".to_string(),
        ]
    }

    pub fn binds(&self) -> Vec<String> {
        let mut binds = vec![
            format!("{}:/workspace", self.worktree_path),
            format!("{}:/home/user/.config/opencode/:ro", self.config_mount_path),
        ];

        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let ssh_path = format!("{}/.ssh", home);
        if Path::new(&ssh_path).exists() {
            binds.push(format!("{}:/home/user/.ssh/:ro", ssh_path));
        }
        let gitconfig_path = format!("{}/.gitconfig", home);
        if Path::new(&gitconfig_path).exists() {
            binds.push(format!("{}:/home/user/.gitconfig:ro", gitconfig_path));
        }

        binds
    }

    pub fn port_bindings(&self) -> HashMap<String, Vec<PortBinding>> {
        let mut bindings = HashMap::new();
        bindings.insert(
            format!("{}/tcp", self.container_port),
            vec![PortBinding {
                host_ip: "127.0.0.1".to_string(),
                host_port: self.host_port.to_string(),
            }],
        );
        bindings
    }

    pub fn env_passthrough(&self) -> Vec<String> {
        self.env_vars
            .iter()
            .filter_map(|key| {
                std::env::var(key)
                    .ok()
                    .map(|val| format!("{}={}", key, val))
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct PortBinding {
    pub host_ip: String,
    pub host_port: String,
}

#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    async fn create_container(&self, config: &ContainerConfig) -> Result<String>;
    async fn start_container(&self, container_id: &str) -> Result<()>;
    async fn stop_container(&self, container_id: &str, timeout_secs: u64) -> Result<()>;
    async fn remove_container(&self, container_id: &str, force: bool) -> Result<()>;
    async fn inspect_container(&self, container_id: &str) -> Result<ContainerInfo>;
    async fn list_containers_by_prefix(&self, prefix: &str) -> Result<Vec<ContainerInfo>>;
}

pub struct DockerRuntime {
    client: bollard::Docker,
}

impl DockerRuntime {
    pub fn new() -> Result<Self> {
        let client = bollard::Docker::connect_with_local_defaults()
            .map_err(|e| anyhow::anyhow!("Failed to connect to Docker: {}", e))?;
        Ok(Self { client })
    }
}

#[async_trait]
impl ContainerRuntime for DockerRuntime {
    async fn create_container(&self, config: &ContainerConfig) -> Result<String> {
        use bollard::container::Config as ContainerCreateConfig;
        use bollard::container::CreateContainerOptions;
        use bollard::models::{HostConfig, PortBinding as BollardPortBinding};

        debug!(
            instance_id = %config.instance_id,
            image = %config.image,
            host_port = config.host_port,
            "Creating Docker container"
        );

        let port_bindings: HashMap<String, Option<Vec<BollardPortBinding>>> = config
            .port_bindings()
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    Some(
                        v.into_iter()
                            .map(|pb| BollardPortBinding {
                                host_ip: Some(pb.host_ip),
                                host_port: Some(pb.host_port),
                            })
                            .collect(),
                    ),
                )
            })
            .collect();

        let host_config = HostConfig {
            binds: Some(config.binds()),
            port_bindings: Some(port_bindings),
            auto_remove: Some(false),
            ..Default::default()
        };

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(
            format!("{}/tcp", config.container_port),
            HashMap::<(), ()>::new(),
        );

        let container_config = ContainerCreateConfig {
            image: Some(config.image.clone()),
            cmd: Some(config.cmd()),
            env: Some(config.env_passthrough()),
            exposed_ports: Some(exposed_ports),
            host_config: Some(host_config),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: config.container_name(),
            platform: None,
        };

        let response = self
            .client
            .create_container(Some(options), container_config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create container: {}", e))?;

        debug!(container_id = %response.id, "Container created");
        Ok(response.id)
    }

    async fn start_container(&self, container_id: &str) -> Result<()> {
        debug!(container_id = %container_id, "Starting container");
        self.client
            .start_container::<String>(container_id, None)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start container: {}", e))?;
        Ok(())
    }

    async fn stop_container(&self, container_id: &str, timeout_secs: u64) -> Result<()> {
        use bollard::container::StopContainerOptions;

        debug!(container_id = %container_id, timeout = timeout_secs, "Stopping container");
        let options = StopContainerOptions {
            t: timeout_secs as i64,
        };
        match self
            .client
            .stop_container(container_id, Some(options))
            .await
        {
            Ok(_) => Ok(()),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 304, ..
            }) => {
                debug!(container_id = %container_id, "Container already stopped");
                Ok(())
            }
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => {
                debug!(container_id = %container_id, "Container not found during stop");
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!("Failed to stop container: {}", e)),
        }
    }

    async fn remove_container(&self, container_id: &str, force: bool) -> Result<()> {
        use bollard::container::RemoveContainerOptions;

        debug!(container_id = %container_id, force = force, "Removing container");
        let options = RemoveContainerOptions {
            force,
            ..Default::default()
        };
        match self
            .client
            .remove_container(container_id, Some(options))
            .await
        {
            Ok(_) => Ok(()),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => {
                debug!(container_id = %container_id, "Container not found during remove");
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!("Failed to remove container: {}", e)),
        }
    }

    async fn inspect_container(&self, container_id: &str) -> Result<ContainerInfo> {
        debug!(container_id = %container_id, "Inspecting container");
        let response = self
            .client
            .inspect_container(container_id, None)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to inspect container: {}", e))?;

        let state = match response.state {
            Some(ref s) => match s.status {
                Some(bollard::models::ContainerStateStatusEnum::RUNNING) => ContainerState::Running,
                Some(bollard::models::ContainerStateStatusEnum::EXITED) => {
                    ContainerState::Exited(s.exit_code.unwrap_or(-1))
                }
                Some(bollard::models::ContainerStateStatusEnum::CREATED) => ContainerState::Created,
                Some(ref other) => ContainerState::Unknown(format!("{:?}", other)),
                None => ContainerState::Unknown("no status".to_string()),
            },
            None => ContainerState::Unknown("no state".to_string()),
        };

        let name = response
            .name
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_string();

        Ok(ContainerInfo {
            id: response.id.unwrap_or_default(),
            name,
            state,
        })
    }

    async fn list_containers_by_prefix(&self, prefix: &str) -> Result<Vec<ContainerInfo>> {
        use bollard::container::ListContainersOptions;

        debug!(prefix = %prefix, "Listing containers by prefix");

        let mut filters = HashMap::new();
        filters.insert("name", vec![prefix]);

        let options = ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        };

        let containers = self
            .client
            .list_containers(Some(options))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list containers: {}", e))?;

        let results = containers
            .into_iter()
            .map(|c| {
                let state = match c.state.as_deref() {
                    Some("running") => ContainerState::Running,
                    Some("exited") => ContainerState::Exited(-1),
                    Some("created") => ContainerState::Created,
                    Some(other) => ContainerState::Unknown(other.to_string()),
                    None => ContainerState::Unknown("unknown".to_string()),
                };
                let name = c
                    .names
                    .and_then(|n| n.first().cloned())
                    .unwrap_or_default()
                    .trim_start_matches('/')
                    .to_string();
                ContainerInfo {
                    id: c.id.unwrap_or_default(),
                    name,
                    state,
                }
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, Clone)]
    pub enum MockAction {
        CreateContainer { config_name: String },
        StartContainer { id: String },
        StopContainer { id: String, timeout: u64 },
        RemoveContainer { id: String, force: bool },
        InspectContainer { id: String },
        ListContainers { prefix: String },
    }

    pub struct MockRuntime {
        pub create_result: Mutex<Result<String, String>>,
        pub start_result: Mutex<Result<(), String>>,
        pub stop_result: Mutex<Result<(), String>>,
        pub remove_result: Mutex<Result<(), String>>,
        pub inspect_result: Mutex<Result<ContainerInfo, String>>,
        pub list_result: Mutex<Result<Vec<ContainerInfo>, String>>,
        pub actions: Mutex<Vec<MockAction>>,
    }

    impl MockRuntime {
        pub fn new() -> Self {
            Self {
                create_result: Mutex::new(Ok("mock-container-id-abc123".to_string())),
                start_result: Mutex::new(Ok(())),
                stop_result: Mutex::new(Ok(())),
                remove_result: Mutex::new(Ok(())),
                inspect_result: Mutex::new(Ok(ContainerInfo {
                    id: "mock-container-id-abc123".to_string(),
                    name: "oc-test".to_string(),
                    state: ContainerState::Running,
                })),
                list_result: Mutex::new(Ok(vec![])),
                actions: Mutex::new(vec![]),
            }
        }

        pub fn with_create_result(self, result: Result<String, String>) -> Self {
            *self.create_result.lock().unwrap() = result;
            self
        }

        pub fn with_stop_result(self, result: Result<(), String>) -> Self {
            *self.stop_result.lock().unwrap() = result;
            self
        }

        pub fn with_inspect_result(self, result: Result<ContainerInfo, String>) -> Self {
            *self.inspect_result.lock().unwrap() = result;
            self
        }

        pub fn with_list_result(self, result: Result<Vec<ContainerInfo>, String>) -> Self {
            *self.list_result.lock().unwrap() = result;
            self
        }

        pub fn recorded_actions(&self) -> Vec<MockAction> {
            self.actions.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl ContainerRuntime for MockRuntime {
        async fn create_container(&self, config: &ContainerConfig) -> Result<String> {
            self.actions
                .lock()
                .unwrap()
                .push(MockAction::CreateContainer {
                    config_name: config.container_name(),
                });
            self.create_result
                .lock()
                .unwrap()
                .clone()
                .map_err(|e| anyhow::anyhow!(e))
        }

        async fn start_container(&self, container_id: &str) -> Result<()> {
            self.actions
                .lock()
                .unwrap()
                .push(MockAction::StartContainer {
                    id: container_id.to_string(),
                });
            self.start_result
                .lock()
                .unwrap()
                .clone()
                .map_err(|e| anyhow::anyhow!(e))
        }

        async fn stop_container(&self, container_id: &str, timeout_secs: u64) -> Result<()> {
            self.actions
                .lock()
                .unwrap()
                .push(MockAction::StopContainer {
                    id: container_id.to_string(),
                    timeout: timeout_secs,
                });
            self.stop_result
                .lock()
                .unwrap()
                .clone()
                .map_err(|e| anyhow::anyhow!(e))
        }

        async fn remove_container(&self, container_id: &str, force: bool) -> Result<()> {
            self.actions
                .lock()
                .unwrap()
                .push(MockAction::RemoveContainer {
                    id: container_id.to_string(),
                    force,
                });
            self.remove_result
                .lock()
                .unwrap()
                .clone()
                .map_err(|e| anyhow::anyhow!(e))
        }

        async fn inspect_container(&self, container_id: &str) -> Result<ContainerInfo> {
            self.actions
                .lock()
                .unwrap()
                .push(MockAction::InspectContainer {
                    id: container_id.to_string(),
                });
            self.inspect_result
                .lock()
                .unwrap()
                .clone()
                .map_err(|e| anyhow::anyhow!(e))
        }

        async fn list_containers_by_prefix(&self, prefix: &str) -> Result<Vec<ContainerInfo>> {
            self.actions
                .lock()
                .unwrap()
                .push(MockAction::ListContainers {
                    prefix: prefix.to_string(),
                });
            self.list_result
                .lock()
                .unwrap()
                .clone()
                .map_err(|e| anyhow::anyhow!(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mock::*;
    use super::*;

    fn test_config() -> ContainerConfig {
        ContainerConfig {
            instance_id: "test-123".to_string(),
            image: "ghcr.io/sst/opencode".to_string(),
            host_port: 4100,
            container_port: 8080,
            worktree_path: "/tmp/projects/.worktrees/my-topic".to_string(),
            config_mount_path: "/home/user/.config/opencode".to_string(),
            env_vars: vec![
                "ANTHROPIC_API_KEY".to_string(),
                "OPENAI_API_KEY".to_string(),
            ],
        }
    }

    #[test]
    fn test_container_name_format() {
        let config = test_config();
        assert_eq!(config.container_name(), "oc-test-123");
    }

    #[test]
    fn test_cmd_includes_port_and_project() {
        let config = test_config();
        let cmd = config.cmd();
        assert_eq!(
            cmd,
            vec![
                "opencode",
                "serve",
                "--port",
                "8080",
                "--project",
                "/workspace"
            ]
        );
    }

    #[test]
    fn test_port_bindings_maps_host_to_container() {
        let config = test_config();
        let bindings = config.port_bindings();

        let key = "8080/tcp";
        assert!(bindings.contains_key(key));
        let bound = &bindings[key];
        assert_eq!(bound.len(), 1);
        assert_eq!(bound[0].host_ip, "127.0.0.1");
        assert_eq!(bound[0].host_port, "4100");
    }

    #[test]
    fn test_binds_includes_worktree_rw() {
        let config = test_config();
        let binds = config.binds();
        assert!(binds
            .iter()
            .any(|b| b == "/tmp/projects/.worktrees/my-topic:/workspace"));
    }

    #[test]
    fn test_binds_includes_config_ro() {
        let config = test_config();
        let binds = config.binds();
        assert!(binds
            .iter()
            .any(|b| b == "/home/user/.config/opencode:/home/user/.config/opencode/:ro"));
    }

    #[test]
    fn test_env_passthrough_filters_set_vars() {
        std::env::set_var("ANTHROPIC_API_KEY", "sk-test-key");
        std::env::remove_var("OPENAI_API_KEY");

        let config = test_config();
        let env = config.env_passthrough();

        assert!(env.iter().any(|e| e == "ANTHROPIC_API_KEY=sk-test-key"));
        assert!(!env.iter().any(|e| e.starts_with("OPENAI_API_KEY=")));

        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[tokio::test]
    async fn test_mock_runtime_create_returns_id() {
        let runtime = MockRuntime::new();
        let config = test_config();

        let id = runtime.create_container(&config).await.unwrap();
        assert_eq!(id, "mock-container-id-abc123");
    }

    #[tokio::test]
    async fn test_mock_runtime_records_actions() {
        let runtime = MockRuntime::new();
        let config = test_config();

        runtime.create_container(&config).await.unwrap();
        runtime.start_container("abc123").await.unwrap();
        runtime.stop_container("abc123", 5).await.unwrap();
        runtime.remove_container("abc123", true).await.unwrap();

        let actions = runtime.recorded_actions();
        assert_eq!(actions.len(), 4);

        assert!(
            matches!(&actions[0], MockAction::CreateContainer { config_name } if config_name == "oc-test-123")
        );
        assert!(matches!(&actions[1], MockAction::StartContainer { id } if id == "abc123"));
        assert!(
            matches!(&actions[2], MockAction::StopContainer { id, timeout } if id == "abc123" && *timeout == 5)
        );
        assert!(
            matches!(&actions[3], MockAction::RemoveContainer { id, force } if id == "abc123" && *force)
        );
    }

    #[tokio::test]
    async fn test_mock_runtime_configurable_create_failure() {
        let runtime = MockRuntime::new().with_create_result(Err("image not found".to_string()));
        let config = test_config();

        let result = runtime.create_container(&config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("image not found"));
    }

    #[tokio::test]
    async fn test_mock_runtime_inspect_running() {
        let runtime = MockRuntime::new().with_inspect_result(Ok(ContainerInfo {
            id: "abc123".to_string(),
            name: "oc-test-123".to_string(),
            state: ContainerState::Running,
        }));

        let info = runtime.inspect_container("abc123").await.unwrap();
        assert_eq!(info.state, ContainerState::Running);
        assert_eq!(info.name, "oc-test-123");
    }

    #[tokio::test]
    async fn test_mock_runtime_inspect_exited() {
        let runtime = MockRuntime::new().with_inspect_result(Ok(ContainerInfo {
            id: "abc123".to_string(),
            name: "oc-test-123".to_string(),
            state: ContainerState::Exited(1),
        }));

        let info = runtime.inspect_container("abc123").await.unwrap();
        assert_eq!(info.state, ContainerState::Exited(1));
    }

    #[tokio::test]
    async fn test_mock_runtime_list_filters_by_prefix() {
        let containers = vec![
            ContainerInfo {
                id: "id1".to_string(),
                name: "oc-instance-1".to_string(),
                state: ContainerState::Running,
            },
            ContainerInfo {
                id: "id2".to_string(),
                name: "oc-instance-2".to_string(),
                state: ContainerState::Exited(0),
            },
        ];

        let runtime = MockRuntime::new().with_list_result(Ok(containers));

        let result = runtime.list_containers_by_prefix("oc-").await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "oc-instance-1");
        assert_eq!(result[1].name, "oc-instance-2");

        let actions = runtime.recorded_actions();
        assert!(matches!(&actions[0], MockAction::ListContainers { prefix } if prefix == "oc-"));
    }

    #[tokio::test]
    async fn test_stop_then_remove_sequence() {
        let runtime = MockRuntime::new();

        runtime.stop_container("abc123", 5).await.unwrap();
        runtime.remove_container("abc123", true).await.unwrap();

        let actions = runtime.recorded_actions();
        assert_eq!(actions.len(), 2);
        assert!(matches!(&actions[0], MockAction::StopContainer { .. }));
        assert!(matches!(&actions[1], MockAction::RemoveContainer { .. }));
    }

    #[tokio::test]
    async fn test_mock_runtime_stop_failure() {
        let runtime = MockRuntime::new().with_stop_result(Err("container not found".to_string()));

        let result = runtime.stop_container("abc123", 5).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_container_config_custom_port() {
        let config = ContainerConfig {
            instance_id: "custom".to_string(),
            image: "custom/image:latest".to_string(),
            host_port: 9999,
            container_port: 3000,
            worktree_path: "/tmp/work".to_string(),
            config_mount_path: "/tmp/config".to_string(),
            env_vars: vec![],
        };

        assert_eq!(config.container_name(), "oc-custom");
        let cmd = config.cmd();
        assert!(cmd.contains(&"3000".to_string()));

        let bindings = config.port_bindings();
        assert!(bindings.contains_key("3000/tcp"));
        assert_eq!(bindings["3000/tcp"][0].host_port, "9999");
    }

    #[test]
    fn test_container_state_equality() {
        assert_eq!(ContainerState::Running, ContainerState::Running);
        assert_eq!(ContainerState::Exited(0), ContainerState::Exited(0));
        assert_ne!(ContainerState::Exited(0), ContainerState::Exited(1));
        assert_ne!(ContainerState::Running, ContainerState::Created);
    }
}
