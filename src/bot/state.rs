use crate::config::Config;
use crate::forum::TopicStore;
use crate::orchestrator::manager::InstanceManager;
use crate::orchestrator::store::OrchestratorStore;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

pub struct BotState {
    pub orchestrator_store: Arc<Mutex<OrchestratorStore>>,
    pub topic_store: Arc<Mutex<TopicStore>>,
    pub config: Arc<Config>,
    pub instance_manager: Arc<InstanceManager>,
    pub bot_start_time: Instant,
}

impl BotState {
    pub fn new(
        orchestrator_store: OrchestratorStore,
        topic_store: TopicStore,
        config: Config,
        instance_manager: InstanceManager,
        bot_start_time: Instant,
    ) -> Self {
        Self {
            orchestrator_store: Arc::new(Mutex::new(orchestrator_store)),
            topic_store: Arc::new(Mutex::new(topic_store)),
            config: Arc::new(config),
            instance_manager: Arc::new(instance_manager),
            bot_start_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::manager::InstanceManager;
    use crate::orchestrator::port_pool::PortPool;
    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::TempDir;

    async fn create_test_config() -> (Config, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            telegram_bot_token: "test_token".to_string(),
            telegram_chat_id: -1001234567890,
            telegram_allowed_users: vec![],
            handle_general_topic: true,
            opencode_path: PathBuf::from("opencode"),
            opencode_max_instances: 10,
            opencode_idle_timeout: Duration::from_secs(1800),
            opencode_port_start: 4100,
            opencode_port_pool_size: 100,
            opencode_health_check_interval: Duration::from_secs(30),
            opencode_startup_timeout: Duration::from_secs(60),
            orchestrator_db_path: temp_dir.path().join("orchestrator.db"),
            topic_db_path: temp_dir.path().join("topics.db"),
            log_db_path: temp_dir.path().join("logs.db"),
            project_base_path: temp_dir.path().to_path_buf(),
            auto_create_project_dirs: true,
            api_port: 4200,
            api_key: None,
        };
        (config, temp_dir)
    }

    #[tokio::test]
    async fn test_bot_state_construction() {
        let (config, _temp_dir) = create_test_config().await;

        let orchestrator_store = OrchestratorStore::new(&config.orchestrator_db_path)
            .await
            .unwrap();
        let topic_store = TopicStore::new(&config.topic_db_path).await.unwrap();

        let store_for_manager = orchestrator_store.clone();
        let port_pool = PortPool::new(4100, 10);
        let instance_manager =
            InstanceManager::new(Arc::new(config.clone()), store_for_manager, port_pool)
                .await
                .unwrap();
        let bot_start_time = Instant::now();

        let state = BotState::new(
            orchestrator_store,
            topic_store,
            config.clone(),
            instance_manager,
            bot_start_time,
        );

        assert_eq!(state.config.telegram_chat_id, -1001234567890);
        assert_eq!(state.config.opencode_max_instances, 10);
    }

    #[tokio::test]
    async fn test_bot_state_stores_accessible() {
        let (config, _temp_dir) = create_test_config().await;

        let orchestrator_store = OrchestratorStore::new(&config.orchestrator_db_path)
            .await
            .unwrap();
        let topic_store = TopicStore::new(&config.topic_db_path).await.unwrap();

        let store_for_manager = orchestrator_store.clone();
        let port_pool = PortPool::new(4100, 10);
        let instance_manager =
            InstanceManager::new(Arc::new(config.clone()), store_for_manager, port_pool)
                .await
                .unwrap();
        let bot_start_time = Instant::now();

        let state = BotState::new(
            orchestrator_store,
            topic_store,
            config,
            instance_manager,
            bot_start_time,
        );

        let _orchestrator = state.orchestrator_store.lock().await;
        let _topics = state.topic_store.lock().await;
    }

    #[tokio::test]
    async fn test_bot_state_config_is_arc() {
        let (config, _temp_dir) = create_test_config().await;

        let orchestrator_store = OrchestratorStore::new(&config.orchestrator_db_path)
            .await
            .unwrap();
        let topic_store = TopicStore::new(&config.topic_db_path).await.unwrap();

        let store_for_manager = orchestrator_store.clone();
        let port_pool = PortPool::new(4100, 10);
        let instance_manager =
            InstanceManager::new(Arc::new(config.clone()), store_for_manager, port_pool)
                .await
                .unwrap();
        let bot_start_time = Instant::now();

        let state = BotState::new(
            orchestrator_store,
            topic_store,
            config,
            instance_manager,
            bot_start_time,
        );

        let config_clone = Arc::clone(&state.config);
        assert_eq!(config_clone.telegram_chat_id, state.config.telegram_chat_id);
    }
}
