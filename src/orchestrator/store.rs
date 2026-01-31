use crate::db::init_orchestrator_db;
use crate::types::instance::{InstanceInfo, InstanceState};
use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::path::Path;

#[cfg(test)]
use crate::types::instance::InstanceType;

#[derive(Clone)]
pub struct OrchestratorStore {
    pool: SqlitePool,
}

impl OrchestratorStore {
    pub async fn new(db_path: &Path) -> Result<Self> {
        let pool = init_orchestrator_db(db_path).await?;
        Ok(Self { pool })
    }

    pub async fn save_instance(
        &self,
        instance: &InstanceInfo,
        session_id: Option<&str>,
    ) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;

        let existing = sqlx::query("SELECT created_at FROM instances WHERE id = ?")
            .bind(&instance.id)
            .fetch_optional(&self.pool)
            .await?;

        let created_at = if let Some(row) = existing {
            row.get::<i64, _>("created_at")
        } else {
            now
        };

        sqlx::query(
            "INSERT OR REPLACE INTO instances 
             (id, project_path, port, state, instance_type, session_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&instance.id)
        .bind(&instance.project_path)
        .bind(instance.port as i64)
        .bind(serde_json::to_string(&instance.state)?)
        .bind(serde_json::to_string(&instance.instance_type)?)
        .bind(session_id)
        .bind(created_at)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_instance(&self, id: &str) -> Result<Option<InstanceInfo>> {
        let row = sqlx::query("SELECT * FROM instances WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|r| self.row_to_instance(r)).transpose()
    }

    #[allow(dead_code)]
    // Used by future: port-based instance lookup feature
    pub async fn get_instance_by_port(&self, port: u16) -> Result<Option<InstanceInfo>> {
        let row = sqlx::query("SELECT * FROM instances WHERE port = ?")
            .bind(port as i64)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|r| self.row_to_instance(r)).transpose()
    }

    pub async fn get_instance_by_path(&self, path: &str) -> Result<Option<InstanceInfo>> {
        let row = sqlx::query("SELECT * FROM instances WHERE project_path = ?")
            .bind(path)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|r| self.row_to_instance(r)).transpose()
    }

    pub async fn get_all_instances(&self) -> Result<Vec<InstanceInfo>> {
        let rows = sqlx::query("SELECT * FROM instances ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(|r| self.row_to_instance(r)).collect()
    }

    pub async fn update_state(&self, id: &str, state: InstanceState) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as i64;

        sqlx::query("UPDATE instances SET state = ?, updated_at = ? WHERE id = ?")
            .bind(serde_json::to_string(&state)?)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_instance(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM instances WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    // Used by future: active instance counting feature
    pub async fn get_active_count(&self) -> Result<usize> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM instances 
             WHERE state != ?",
        )
        .bind(serde_json::to_string(&InstanceState::Stopped)?)
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = row.get("count");
        Ok(count as usize)
    }

    fn row_to_instance(&self, row: sqlx::sqlite::SqliteRow) -> Result<InstanceInfo> {
        let state_str: String = row.get("state");
        let type_str: String = row.get("instance_type");
        let port: i64 = row.get("port");

        Ok(InstanceInfo {
            id: row.get("id"),
            project_path: row.get("project_path"),
            port: port as u16,
            state: serde_json::from_str(&state_str)?,
            instance_type: serde_json::from_str(&type_str)?,
            pid: None,
            started_at: None,
            stopped_at: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_instance(id: &str, port: u16, path: &str) -> InstanceInfo {
        InstanceInfo {
            id: id.to_string(),
            project_path: path.to_string(),
            port,
            state: InstanceState::Running,
            instance_type: InstanceType::Managed,
            pid: None,
            started_at: None,
            stopped_at: None,
        }
    }

    #[tokio::test]
    async fn test_new_creates_store_with_pool() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let _store = OrchestratorStore::new(&db_path).await.unwrap();

        assert!(db_path.exists());
    }

    #[tokio::test]
    async fn test_save_instance_inserts_new_instance() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let instance = create_test_instance("test-1", 4100, "/test/path");
        store
            .save_instance(&instance, Some("ses_test-1"))
            .await
            .unwrap();

        let retrieved = store.get_instance("test-1").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, "test-1");
        assert_eq!(retrieved.port, 4100);
        assert_eq!(retrieved.project_path, "/test/path");
    }

    #[tokio::test]
    async fn test_save_instance_updates_existing_instance() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let mut instance = create_test_instance("test-1", 4100, "/test/path");
        store
            .save_instance(&instance, Some("ses_test-1"))
            .await
            .unwrap();

        instance.port = 4101;
        instance.state = InstanceState::Stopped;
        store
            .save_instance(&instance, Some("ses_test-1"))
            .await
            .unwrap();

        let retrieved = store.get_instance("test-1").await.unwrap().unwrap();
        assert_eq!(retrieved.port, 4101);
        assert_eq!(retrieved.state, InstanceState::Stopped);
    }

    #[tokio::test]
    async fn test_get_instance_returns_none_when_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let result = store.get_instance("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_instance_by_port_finds_instance() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let instance = create_test_instance("test-1", 4100, "/test/path");
        store
            .save_instance(&instance, Some("ses_test-1"))
            .await
            .unwrap();

        let retrieved = store.get_instance_by_port(4100).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test-1");
    }

    #[tokio::test]
    async fn test_get_instance_by_port_returns_none_when_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let result = store.get_instance_by_port(9999).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_instance_by_path_finds_instance() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let instance = create_test_instance("test-1", 4100, "/test/path");
        store
            .save_instance(&instance, Some("ses_test-1"))
            .await
            .unwrap();

        let retrieved = store.get_instance_by_path("/test/path").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test-1");
    }

    #[tokio::test]
    async fn test_get_instance_by_path_returns_none_when_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let result = store
            .get_instance_by_path("/nonexistent/path")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_all_instances_returns_empty_when_no_instances() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let instances = store.get_all_instances().await.unwrap();
        assert_eq!(instances.len(), 0);
    }

    #[tokio::test]
    async fn test_get_all_instances_returns_all_instances() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let instance1 = create_test_instance("test-1", 4100, "/test/path1");
        let instance2 = create_test_instance("test-2", 4101, "/test/path2");
        let instance3 = create_test_instance("test-3", 4102, "/test/path3");

        store
            .save_instance(&instance1, Some("ses_1"))
            .await
            .unwrap();
        store
            .save_instance(&instance2, Some("ses_2"))
            .await
            .unwrap();
        store
            .save_instance(&instance3, Some("ses_3"))
            .await
            .unwrap();

        let instances = store.get_all_instances().await.unwrap();
        assert_eq!(instances.len(), 3);
    }

    #[tokio::test]
    async fn test_update_state_changes_instance_state() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let instance = create_test_instance("test-1", 4100, "/test/path");
        store
            .save_instance(&instance, Some("ses_test-1"))
            .await
            .unwrap();

        store
            .update_state("test-1", InstanceState::Stopping)
            .await
            .unwrap();

        let retrieved = store.get_instance("test-1").await.unwrap().unwrap();
        assert_eq!(retrieved.state, InstanceState::Stopping);
    }

    #[tokio::test]
    async fn test_update_state_updates_timestamp() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let instance = create_test_instance("test-1", 4100, "/test/path");
        store
            .save_instance(&instance, Some("ses_test-1"))
            .await
            .unwrap();

        let row1 = sqlx::query("SELECT updated_at FROM instances WHERE id = ?")
            .bind("test-1")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        let original_updated_at: i64 = row1.get("updated_at");

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        store
            .update_state("test-1", InstanceState::Stopped)
            .await
            .unwrap();

        let row2 = sqlx::query("SELECT updated_at FROM instances WHERE id = ?")
            .bind("test-1")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        let new_updated_at: i64 = row2.get("updated_at");

        assert!(new_updated_at > original_updated_at);
    }

    #[tokio::test]
    async fn test_delete_instance_removes_instance() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let instance = create_test_instance("test-1", 4100, "/test/path");
        store
            .save_instance(&instance, Some("ses_test-1"))
            .await
            .unwrap();

        store.delete_instance("test-1").await.unwrap();

        let retrieved = store.get_instance("test-1").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_delete_instance_does_not_error_when_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let result = store.delete_instance("nonexistent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_active_count_returns_zero_when_no_instances() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let count = store.get_active_count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_get_active_count_excludes_stopped_instances() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let mut instance1 = create_test_instance("test-1", 4100, "/test/path1");
        instance1.state = InstanceState::Running;
        let mut instance2 = create_test_instance("test-2", 4101, "/test/path2");
        instance2.state = InstanceState::Stopped;
        let mut instance3 = create_test_instance("test-3", 4102, "/test/path3");
        instance3.state = InstanceState::Starting;

        store
            .save_instance(&instance1, Some("ses_1"))
            .await
            .unwrap();
        store
            .save_instance(&instance2, Some("ses_2"))
            .await
            .unwrap();
        store
            .save_instance(&instance3, Some("ses_3"))
            .await
            .unwrap();

        let count = store.get_active_count().await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_concurrent_access_does_not_cause_errors() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let mut handles = vec![];
        for i in 0..10 {
            let store_clone = OrchestratorStore::new(&db_path).await.unwrap();
            let handle = tokio::spawn(async move {
                let instance = create_test_instance(
                    &format!("test-{}", i),
                    4100 + i as u16,
                    &format!("/test/path{}", i),
                );
                store_clone
                    .save_instance(&instance, Some(&format!("ses_{}", i)))
                    .await
                    .unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let instances = store.get_all_instances().await.unwrap();
        assert_eq!(instances.len(), 10);
    }

    #[tokio::test]
    async fn test_save_preserves_all_instance_types() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let types = [
            InstanceType::Managed,
            InstanceType::Discovered,
            InstanceType::External,
        ];

        for (i, instance_type) in types.iter().enumerate() {
            let mut instance = create_test_instance(
                &format!("test-{}", i),
                4100 + i as u16,
                &format!("/test/path{}", i),
            );
            instance.instance_type = instance_type.clone();
            store
                .save_instance(&instance, Some(&format!("ses_{}", i)))
                .await
                .unwrap();
        }

        let instances = store.get_all_instances().await.unwrap();
        assert_eq!(instances.len(), 3);

        let managed_count = instances
            .iter()
            .filter(|i| i.instance_type == InstanceType::Managed)
            .count();
        let discovered_count = instances
            .iter()
            .filter(|i| i.instance_type == InstanceType::Discovered)
            .count();
        let external_count = instances
            .iter()
            .filter(|i| i.instance_type == InstanceType::External)
            .count();

        assert_eq!(managed_count, 1);
        assert_eq!(discovered_count, 1);
        assert_eq!(external_count, 1);
    }

    #[tokio::test]
    async fn test_save_preserves_all_instance_states() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let states = [
            InstanceState::Starting,
            InstanceState::Running,
            InstanceState::Stopping,
            InstanceState::Stopped,
            InstanceState::Error,
        ];

        for (i, state) in states.iter().enumerate() {
            let mut instance = create_test_instance(
                &format!("test-{}", i),
                4100 + i as u16,
                &format!("/test/path{}", i),
            );
            instance.state = state.clone();
            store
                .save_instance(&instance, Some(&format!("ses_{}", i)))
                .await
                .unwrap();
        }

        let instances = store.get_all_instances().await.unwrap();
        assert_eq!(instances.len(), 5);
    }

    #[tokio::test]
    async fn test_save_handles_none_session_id() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let instance = create_test_instance("test-1", 4100, "/test/path");
        store.save_instance(&instance, None).await.unwrap();

        let retrieved = store.get_instance("test-1").await.unwrap().unwrap();
        assert_eq!(retrieved.id, "test-1");
    }

    #[tokio::test]
    async fn test_save_preserves_created_at_on_update() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = OrchestratorStore::new(&db_path).await.unwrap();

        let instance = create_test_instance("test-1", 4100, "/test/path");
        store
            .save_instance(&instance, Some("ses_test-1"))
            .await
            .unwrap();

        let row1 = sqlx::query("SELECT created_at FROM instances WHERE id = ?")
            .bind("test-1")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        let original_created_at: i64 = row1.get("created_at");

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let mut updated_instance = instance.clone();
        updated_instance.port = 4101;
        store
            .save_instance(&updated_instance, Some("ses_test-1"))
            .await
            .unwrap();

        let row2 = sqlx::query("SELECT created_at FROM instances WHERE id = ?")
            .bind("test-1")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        let new_created_at: i64 = row2.get("created_at");

        assert_eq!(original_created_at, new_created_at);
    }
}
