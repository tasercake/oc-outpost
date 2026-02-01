use crate::bot::{BotState, Command};
use crate::types::error::{OutpostError, Result};
use crate::types::forum::TopicMapping;
use crate::types::instance::{InstanceState, InstanceType};
use reqwest;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use teloxide::prelude::*;
use teloxide::types::MessageId;
use tracing::{debug, warn};

/// Check if message is in General topic (thread_id is None or ThreadId(MessageId(1)))
fn is_general_topic(msg: &Message) -> bool {
    msg.thread_id.is_none() || (msg.thread_id.map(|id| id.0) == Some(MessageId(1)))
}

/// Why a mapping is considered stale and should be cleaned up
#[derive(Debug, Clone, PartialEq, Eq)]
enum StalenessReason {
    /// Instance is in Stopped or Error state in the orchestrator store
    InstanceStopped,
    /// Instance exists in DB but health check (port liveness) failed
    InstanceUnhealthy,
    /// Mapping references an instance_id that doesn't exist in orchestrator store
    OrphanedMapping,
    /// Mapping has no instance_id and no session_id (incomplete setup)
    IncompleteMapping,
    /// Mapping hasn't been updated in over 7 days (fallback heuristic)
    Inactive { days: u64 },
}

impl std::fmt::Display for StalenessReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StalenessReason::InstanceStopped => write!(f, "instance stopped"),
            StalenessReason::InstanceUnhealthy => write!(f, "instance unreachable"),
            StalenessReason::OrphanedMapping => write!(f, "instance not found"),
            StalenessReason::IncompleteMapping => write!(f, "incomplete setup"),
            StalenessReason::Inactive { days } => write!(f, "inactive for {} days", days),
        }
    }
}

struct CleanupCandidate {
    mapping: TopicMapping,
    reason: StalenessReason,
}

fn project_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn build_cleanup_response(candidates: &[CleanupCandidate]) -> String {
    if candidates.is_empty() {
        "🧹 Cleanup Complete\n\nNo stale mappings found. All connections are healthy.".to_string()
    } else {
        let mut msg = format!(
            "🧹 Cleanup Complete\n\nCleared {} stale mapping(s):\n",
            candidates.len()
        );
        for candidate in candidates {
            let project_name = project_name_from_path(&candidate.mapping.project_path);
            msg.push_str(&format!("• {} — {}\n", project_name, candidate.reason));
        }
        msg.trim_end().to_string()
    }
}

async fn find_cleanup_candidates(state: &BotState) -> Result<Vec<CleanupCandidate>> {
    let topic_store = state.topic_store.lock().await;
    let mappings = topic_store
        .get_all_mappings()
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?;
    drop(topic_store);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| OutpostError::io_error(e.to_string()))?
        .as_secs() as i64;
    let stale_threshold = 7 * 24 * 60 * 60;
    let health_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default();

    let mut candidates = Vec::new();

    for mapping in mappings {
        if let Some(instance_id) = &mapping.instance_id {
            let store = state.orchestrator_store.lock().await;
            let instance = store
                .get_instance(instance_id)
                .await
                .map_err(|e| OutpostError::database_error(e.to_string()))?;
            drop(store);

            match instance {
                None => {
                    candidates.push(CleanupCandidate {
                        mapping,
                        reason: StalenessReason::OrphanedMapping,
                    });
                }
                Some(instance_info) => match instance_info.state {
                    InstanceState::Stopped | InstanceState::Error => {
                        candidates.push(CleanupCandidate {
                            mapping,
                            reason: StalenessReason::InstanceStopped,
                        });
                    }
                    InstanceState::Running | InstanceState::Starting | InstanceState::Stopping => {
                        let url = format!("http://localhost:{}/global/health", instance_info.port);
                        let is_healthy = match health_client.get(url).send().await {
                            Ok(resp) => resp.status().is_success(),
                            Err(_) => false,
                        };
                        if !is_healthy {
                            candidates.push(CleanupCandidate {
                                mapping,
                                reason: StalenessReason::InstanceUnhealthy,
                            });
                        }
                    }
                },
            }

            continue;
        }

        if mapping.session_id.is_none() {
            candidates.push(CleanupCandidate {
                mapping,
                reason: StalenessReason::IncompleteMapping,
            });
            continue;
        }

        let age_secs = now.saturating_sub(mapping.updated_at);
        if age_secs > stale_threshold {
            let days = (age_secs / (24 * 60 * 60)) as u64;
            candidates.push(CleanupCandidate {
                mapping,
                reason: StalenessReason::Inactive { days },
            });
        }
    }

    Ok(candidates)
}

async fn cleanup_candidate(state: &BotState, candidate: CleanupCandidate) -> Result<()> {
    if let Some(instance_id) = &candidate.mapping.instance_id {
        let store = state.orchestrator_store.lock().await;
        let instance = store
            .get_instance(instance_id)
            .await
            .map_err(|e| OutpostError::database_error(e.to_string()))?;
        drop(store);

        if let Some(instance_info) = instance {
            if instance_info.instance_type == InstanceType::Managed {
                debug!(instance_id = %instance_id, "Stopping stale managed instance");
                if let Err(e) = state.instance_manager.stop_instance(instance_id).await {
                    warn!(
                        instance_id = %instance_id,
                        error = %e,
                        "Failed to stop stale managed instance"
                    );
                }
            }
        }
    }

    let topic_store = state.topic_store.lock().await;
    topic_store
        .delete_mapping(candidate.mapping.topic_id)
        .await
        .map_err(|e| OutpostError::database_error(e.to_string()))?;
    drop(topic_store);
    debug!(
        topic_id = candidate.mapping.topic_id,
        "Stale mapping deleted"
    );

    let project_path = Path::new(&candidate.mapping.project_path);
    let base_path = &state.config.project_base_path;
    if project_path.starts_with(base_path) && project_path != base_path {
        if project_path.is_dir() {
            match std::fs::remove_dir_all(project_path) {
                Ok(()) => {
                    debug!(
                        path = %candidate.mapping.project_path,
                        "Removed stale project directory"
                    );
                }
                Err(e) => {
                    warn!(
                        path = %candidate.mapping.project_path,
                        error = %e,
                        "Failed to remove stale project directory"
                    );
                }
            }
        }
    }

    Ok(())
}

pub async fn handle_clear(
    bot: Bot,
    msg: Message,
    _cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    debug!(
        chat_id = msg.chat.id.0,
        topic_id = ?msg.thread_id.map(|t| t.0 .0),
        sender_id = ?msg.from.as_ref().map(|u| u.id.0),
        "Handling /clear"
    );
    let chat_id = msg.chat.id;

    if !is_general_topic(&msg) {
        bot.send_message(
            chat_id,
            "The /clear command can only be used in the General topic.",
        )
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;
        return Ok(());
    }

    let candidates = find_cleanup_candidates(&state).await?;
    debug!(stale_count = candidates.len(), "Stale mappings found");

    let response = build_cleanup_response(&candidates);

    for candidate in candidates {
        cleanup_candidate(&state, candidate).await?;
    }
    debug!("Clear operation complete");

    bot.send_message(chat_id, response)
        .await
        .map_err(|e| OutpostError::telegram_error(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::forum::TopicStore;
    use crate::orchestrator::store::OrchestratorStore;
    use crate::types::forum::TopicMapping;
    use crate::types::instance::{InstanceInfo, InstanceState};
    use serde_json::json;
    use std::path::PathBuf;
    use tempfile::TempDir;

    async fn create_test_state() -> (BotState, TempDir) {
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

        let orchestrator_store = OrchestratorStore::new(&config.orchestrator_db_path)
            .await
            .unwrap();
        let topic_store = TopicStore::new(&config.topic_db_path).await.unwrap();

        let store_for_manager = orchestrator_store.clone();
        let port_pool = crate::orchestrator::port_pool::PortPool::new(4100, 10);
        let instance_manager = crate::orchestrator::manager::InstanceManager::new(
            std::sync::Arc::new(config.clone()),
            store_for_manager,
            port_pool,
        )
        .await
        .unwrap();
        let bot_start_time = std::time::Instant::now();

        let state = BotState::new(
            orchestrator_store,
            topic_store,
            config,
            instance_manager,
            bot_start_time,
        );
        (state, temp_dir)
    }

    fn message_with_thread_id(thread_id: Option<i32>) -> Message {
        let mut value = json!({
            "message_id": 1,
            "date": 0,
            "chat": { "id": 1, "type": "supergroup" }
        });
        if let Some(id) = thread_id {
            value["message_thread_id"] = json!(id);
        }
        serde_json::from_value(value).unwrap()
    }

    #[tokio::test]
    async fn test_clear_with_no_stale_mappings() {
        let (state, _temp_dir) = create_test_state().await;

        let candidates = find_cleanup_candidates(&state).await.unwrap();
        assert!(candidates.is_empty());

        let response = build_cleanup_response(&candidates);
        assert_eq!(
            response,
            "🧹 Cleanup Complete\n\nNo stale mappings found. All connections are healthy."
        );
    }

    #[tokio::test]
    async fn test_clear_general_topic_detection() {
        let general_message = message_with_thread_id(None);
        assert!(is_general_topic(&general_message));

        let general_thread = message_with_thread_id(Some(1));
        assert!(is_general_topic(&general_thread));

        let non_general_thread = message_with_thread_id(Some(2));
        assert!(!is_general_topic(&non_general_thread));
    }

    #[tokio::test]
    async fn test_staleness_reason_display() {
        assert_eq!(
            StalenessReason::InstanceStopped.to_string(),
            "instance stopped"
        );
        assert_eq!(
            StalenessReason::InstanceUnhealthy.to_string(),
            "instance unreachable"
        );
        assert_eq!(
            StalenessReason::OrphanedMapping.to_string(),
            "instance not found"
        );
        assert_eq!(
            StalenessReason::IncompleteMapping.to_string(),
            "incomplete setup"
        );
        assert_eq!(
            StalenessReason::Inactive { days: 9 }.to_string(),
            "inactive for 9 days"
        );
    }

    #[tokio::test]
    async fn test_clear_with_stopped_instance() {
        let (state, _temp_dir) = create_test_state().await;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mapping = TopicMapping {
            topic_id: 100,
            chat_id: -1001234567890,
            project_path: "/test/old-project".to_string(),
            session_id: Some("ses_old".to_string()),
            instance_id: Some("inst_managed".to_string()),
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: now,
            updated_at: now,
        };

        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        let instance = InstanceInfo {
            id: "inst_managed".to_string(),
            state: InstanceState::Stopped,
            instance_type: InstanceType::Managed,
            project_path: "/test/old-project".to_string(),
            port: 4100,
            pid: Some(12345),
            started_at: Some(now),
            stopped_at: Some(now),
        };

        let store = state.orchestrator_store.lock().await;
        store
            .save_instance(&instance, Some("ses_old"))
            .await
            .unwrap();
        drop(store);

        let candidates = find_cleanup_candidates(&state).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].mapping.project_path, "/test/old-project");
        assert_eq!(candidates[0].reason, StalenessReason::InstanceStopped);
    }

    #[tokio::test]
    async fn test_clear_with_orphaned_mapping() {
        let (state, _temp_dir) = create_test_state().await;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mapping = TopicMapping {
            topic_id: 200,
            chat_id: -1001234567890,
            project_path: "/test/missing-instance".to_string(),
            session_id: Some("ses_missing".to_string()),
            instance_id: Some("inst_nonexistent".to_string()),
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: now,
            updated_at: now,
        };

        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        let candidates = find_cleanup_candidates(&state).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].reason, StalenessReason::OrphanedMapping);
    }

    #[tokio::test]
    async fn test_clear_with_incomplete_mapping() {
        let (state, _temp_dir) = create_test_state().await;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mapping = TopicMapping {
            topic_id: 300,
            chat_id: -1001234567890,
            project_path: "/test/incomplete".to_string(),
            session_id: None,
            instance_id: None,
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: now,
            updated_at: now,
        };

        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        let candidates = find_cleanup_candidates(&state).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].reason, StalenessReason::IncompleteMapping);
    }

    #[tokio::test]
    async fn test_clear_with_inactive_mapping() {
        let (state, _temp_dir) = create_test_state().await;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let old_time = now - (8 * 24 * 60 * 60);

        let mapping = TopicMapping {
            topic_id: 400,
            chat_id: -1001234567890,
            project_path: "/test/inactive".to_string(),
            session_id: Some("ses_inactive".to_string()),
            instance_id: None,
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: old_time,
            updated_at: old_time,
        };

        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        let candidates = find_cleanup_candidates(&state).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].reason, StalenessReason::Inactive { days: 8 });
    }

    #[tokio::test]
    async fn test_clear_removes_project_directory_under_base_path() {
        let (state, temp_dir) = create_test_state().await;

        let project_dir = temp_dir.path().join("stale-project");
        std::fs::create_dir_all(&project_dir).unwrap();
        std::fs::write(project_dir.join("dummy.txt"), "test").unwrap();
        assert!(project_dir.is_dir());

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let old_time = now - (8 * 24 * 60 * 60);

        let mapping = TopicMapping {
            topic_id: 500,
            chat_id: -1001234567890,
            project_path: project_dir.to_string_lossy().to_string(),
            session_id: Some("ses_stale".to_string()),
            instance_id: None,
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: old_time,
            updated_at: old_time,
        };

        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        let candidates = find_cleanup_candidates(&state).await.unwrap();
        assert_eq!(candidates.len(), 1);
        cleanup_candidate(&state, candidates.into_iter().next().unwrap())
            .await
            .unwrap();

        assert!(!project_dir.exists());
    }

    #[tokio::test]
    async fn test_clear_does_not_remove_directory_outside_base_path() {
        let (state, _temp_dir) = create_test_state().await;

        let external_dir = TempDir::new().unwrap();
        let external_path = external_dir.path().join("linked-project");
        std::fs::create_dir_all(&external_path).unwrap();
        assert!(external_path.is_dir());

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let old_time = now - (8 * 24 * 60 * 60);

        let mapping = TopicMapping {
            topic_id: 600,
            chat_id: -1001234567890,
            project_path: external_path.to_string_lossy().to_string(),
            session_id: Some("ses_linked".to_string()),
            instance_id: None,
            streaming_enabled: false,
            topic_name_updated: false,
            created_at: old_time,
            updated_at: old_time,
        };

        let topic_store = state.topic_store.lock().await;
        topic_store.save_mapping(&mapping).await.unwrap();
        drop(topic_store);

        let candidates = find_cleanup_candidates(&state).await.unwrap();
        assert_eq!(candidates.len(), 1);
        cleanup_candidate(&state, candidates.into_iter().next().unwrap())
            .await
            .unwrap();

        assert!(external_path.is_dir());
    }
}
