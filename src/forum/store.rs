use crate::db::init_topics_db;
use crate::types::forum::TopicMapping;
use anyhow::{anyhow, Result};
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::time::Duration;

#[allow(dead_code)]
pub struct TopicStore {
    pool: SqlitePool,
}

impl TopicStore {
    #[allow(dead_code)]
    pub async fn new(db_path: &Path) -> Result<Self> {
        let pool = init_topics_db(db_path).await?;
        Ok(Self { pool })
    }

    #[allow(dead_code)]
    pub async fn save_mapping(&self, mapping: &TopicMapping) -> Result<()> {
        sqlx::query(
            "INSERT INTO topic_mappings 
             (topic_id, chat_id, project_path, session_id, instance_id, 
              streaming_enabled, topic_name_updated, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(topic_id) DO UPDATE SET
                chat_id = excluded.chat_id,
                project_path = excluded.project_path,
                session_id = excluded.session_id,
                instance_id = excluded.instance_id,
                streaming_enabled = excluded.streaming_enabled,
                topic_name_updated = excluded.topic_name_updated,
                updated_at = excluded.updated_at",
        )
        .bind(mapping.topic_id)
        .bind(mapping.chat_id)
        .bind(&mapping.project_path)
        .bind(&mapping.session_id)
        .bind(&mapping.instance_id)
        .bind(if mapping.streaming_enabled { 1 } else { 0 })
        .bind(if mapping.topic_name_updated { 1 } else { 0 })
        .bind(mapping.created_at)
        .bind(mapping.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn get_mapping(&self, topic_id: i32) -> Result<Option<TopicMapping>> {
        let row = sqlx::query(
            "SELECT topic_id, chat_id, project_path, session_id, instance_id,
                    streaming_enabled, topic_name_updated, created_at, updated_at
             FROM topic_mappings WHERE topic_id = ?",
        )
        .bind(topic_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(TopicMapping {
                topic_id: row.get(0),
                chat_id: row.get(1),
                project_path: row.get(2),
                session_id: row.get(3),
                instance_id: row.get(4),
                streaming_enabled: row.get::<i32, _>(5) != 0,
                topic_name_updated: row.get::<i32, _>(6) != 0,
                created_at: row.get(7),
                updated_at: row.get(8),
            })),
            None => Ok(None),
        }
    }

    #[allow(dead_code)]
    pub async fn get_mappings_by_chat(&self, chat_id: i64) -> Result<Vec<TopicMapping>> {
        let rows = sqlx::query(
            "SELECT topic_id, chat_id, project_path, session_id, instance_id,
                    streaming_enabled, topic_name_updated, created_at, updated_at
             FROM topic_mappings WHERE chat_id = ?",
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;

        let mappings = rows
            .into_iter()
            .map(|row| TopicMapping {
                topic_id: row.get(0),
                chat_id: row.get(1),
                project_path: row.get(2),
                session_id: row.get(3),
                instance_id: row.get(4),
                streaming_enabled: row.get::<i32, _>(5) != 0,
                topic_name_updated: row.get::<i32, _>(6) != 0,
                created_at: row.get(7),
                updated_at: row.get(8),
            })
            .collect();

        Ok(mappings)
    }

    #[allow(dead_code)]
    pub async fn get_mapping_by_session(&self, session_id: &str) -> Result<Option<TopicMapping>> {
        let row = sqlx::query(
            "SELECT topic_id, chat_id, project_path, session_id, instance_id,
                    streaming_enabled, topic_name_updated, created_at, updated_at
             FROM topic_mappings WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(TopicMapping {
                topic_id: row.get(0),
                chat_id: row.get(1),
                project_path: row.get(2),
                session_id: row.get(3),
                instance_id: row.get(4),
                streaming_enabled: row.get::<i32, _>(5) != 0,
                topic_name_updated: row.get::<i32, _>(6) != 0,
                created_at: row.get(7),
                updated_at: row.get(8),
            })),
            None => Ok(None),
        }
    }

    #[allow(dead_code)]
    pub async fn update_session(&self, topic_id: i32, session_id: &str) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let result = sqlx::query(
            "UPDATE topic_mappings SET session_id = ?, updated_at = ? WHERE topic_id = ?",
        )
        .bind(session_id)
        .bind(now)
        .bind(topic_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Mapping not found for topic_id {}", topic_id));
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn toggle_streaming(&self, topic_id: i32) -> Result<bool> {
        let current = self
            .get_mapping(topic_id)
            .await?
            .ok_or_else(|| anyhow!("Mapping not found for topic_id {}", topic_id))?;

        let new_value = !current.streaming_enabled;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        sqlx::query(
            "UPDATE topic_mappings SET streaming_enabled = ?, updated_at = ? WHERE topic_id = ?",
        )
        .bind(if new_value { 1 } else { 0 })
        .bind(now)
        .bind(topic_id)
        .execute(&self.pool)
        .await?;

        Ok(new_value)
    }

    #[allow(dead_code)]
    pub async fn mark_topic_name_updated(&self, topic_id: i32) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let result = sqlx::query(
            "UPDATE topic_mappings SET topic_name_updated = 1, updated_at = ? WHERE topic_id = ?",
        )
        .bind(now)
        .bind(topic_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Mapping not found for topic_id {}", topic_id));
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn delete_mapping(&self, topic_id: i32) -> Result<()> {
        sqlx::query("DELETE FROM topic_mappings WHERE topic_id = ?")
            .bind(topic_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn get_stale_mappings(&self, older_than: Duration) -> Result<Vec<TopicMapping>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let threshold = now - older_than.as_secs() as i64;

        let rows = sqlx::query(
            "SELECT topic_id, chat_id, project_path, session_id, instance_id,
                    streaming_enabled, topic_name_updated, created_at, updated_at
             FROM topic_mappings WHERE updated_at < ?",
        )
        .bind(threshold)
        .fetch_all(&self.pool)
        .await?;

        let mappings = rows
            .into_iter()
            .map(|row| TopicMapping {
                topic_id: row.get(0),
                chat_id: row.get(1),
                project_path: row.get(2),
                session_id: row.get(3),
                instance_id: row.get(4),
                streaming_enabled: row.get::<i32, _>(5) != 0,
                topic_name_updated: row.get::<i32, _>(6) != 0,
                created_at: row.get(7),
                updated_at: row.get(8),
            })
            .collect();

        Ok(mappings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_mapping(topic_id: i32, chat_id: i64) -> TopicMapping {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        TopicMapping {
            topic_id,
            chat_id,
            project_path: "/test/project".to_string(),
            session_id: None,
            instance_id: None,
            streaming_enabled: true,
            topic_name_updated: false,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn test_new_creates_store_with_valid_db() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");

        let _store = TopicStore::new(&db_path).await.unwrap();

        // Verify database file was created
        assert!(db_path.exists());
    }

    #[tokio::test]
    async fn test_save_mapping_inserts_new_mapping() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let mapping = create_test_mapping(123, -1001234567890);
        store.save_mapping(&mapping).await.unwrap();

        // Verify mapping was saved
        let retrieved = store.get_mapping(123).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.topic_id, 123);
        assert_eq!(retrieved.chat_id, -1001234567890);
        assert_eq!(retrieved.project_path, "/test/project");
    }

    #[tokio::test]
    async fn test_save_mapping_updates_existing_mapping() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let mut mapping = create_test_mapping(456, -1009876543210);
        store.save_mapping(&mapping).await.unwrap();

        mapping.project_path = "/updated/path".to_string();
        mapping.session_id = Some("session-123".to_string());
        store.save_mapping(&mapping).await.unwrap();

        // Verify mapping was updated
        let retrieved = store.get_mapping(456).await.unwrap().unwrap();
        assert_eq!(retrieved.project_path, "/updated/path");
        assert_eq!(retrieved.session_id, Some("session-123".to_string()));
    }

    #[tokio::test]
    async fn test_get_mapping_returns_none_for_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let result = store.get_mapping(999).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_mappings_by_chat_returns_all_for_chat() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let chat_id = -1001111111111;
        let mapping1 = create_test_mapping(100, chat_id);
        let mapping2 = create_test_mapping(200, chat_id);
        let mapping3 = create_test_mapping(300, -1002222222222); // Different chat

        store.save_mapping(&mapping1).await.unwrap();
        store.save_mapping(&mapping2).await.unwrap();
        store.save_mapping(&mapping3).await.unwrap();

        let results = store.get_mappings_by_chat(chat_id).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|m| m.topic_id == 100));
        assert!(results.iter().any(|m| m.topic_id == 200));
    }

    #[tokio::test]
    async fn test_get_mappings_by_chat_returns_empty_for_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let results = store.get_mappings_by_chat(-1009999999999).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_get_mapping_by_session_finds_mapping() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let mut mapping = create_test_mapping(789, -1003333333333);
        mapping.session_id = Some("session-abc".to_string());
        store.save_mapping(&mapping).await.unwrap();

        let result = store.get_mapping_by_session("session-abc").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().topic_id, 789);
    }

    #[tokio::test]
    async fn test_get_mapping_by_session_returns_none_for_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let result = store.get_mapping_by_session("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_session_updates_session_id() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let mapping = create_test_mapping(555, -1004444444444);
        store.save_mapping(&mapping).await.unwrap();

        store.update_session(555, "new-session-id").await.unwrap();

        let retrieved = store.get_mapping(555).await.unwrap().unwrap();
        assert_eq!(retrieved.session_id, Some("new-session-id".to_string()));
    }

    #[tokio::test]
    async fn test_update_session_fails_for_nonexistent_mapping() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let result = store.update_session(999, "session-id").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_toggle_streaming_flips_boolean() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let mut mapping = create_test_mapping(666, -1005555555555);
        mapping.streaming_enabled = true;
        store.save_mapping(&mapping).await.unwrap();

        // Toggle to false
        let new_value = store.toggle_streaming(666).await.unwrap();
        assert_eq!(new_value, false);

        // Verify it was persisted
        let retrieved = store.get_mapping(666).await.unwrap().unwrap();
        assert_eq!(retrieved.streaming_enabled, false);

        // Toggle back to true
        let new_value = store.toggle_streaming(666).await.unwrap();
        assert_eq!(new_value, true);

        let retrieved = store.get_mapping(666).await.unwrap().unwrap();
        assert_eq!(retrieved.streaming_enabled, true);
    }

    #[tokio::test]
    async fn test_toggle_streaming_fails_for_nonexistent_mapping() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let result = store.toggle_streaming(999).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mark_topic_name_updated_sets_flag() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let mut mapping = create_test_mapping(777, -1006666666666);
        mapping.topic_name_updated = false;
        store.save_mapping(&mapping).await.unwrap();

        store.mark_topic_name_updated(777).await.unwrap();

        let retrieved = store.get_mapping(777).await.unwrap().unwrap();
        assert_eq!(retrieved.topic_name_updated, true);
    }

    #[tokio::test]
    async fn test_mark_topic_name_updated_fails_for_nonexistent_mapping() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let result = store.mark_topic_name_updated(999).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_mapping_removes_mapping() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let mapping = create_test_mapping(888, -1007777777777);
        store.save_mapping(&mapping).await.unwrap();

        store.delete_mapping(888).await.unwrap();

        let result = store.get_mapping(888).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_mapping_succeeds_for_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        // Should not error even if mapping doesn't exist
        let result = store.delete_mapping(999).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_stale_mappings_returns_old_mappings() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Create old mapping (2 hours ago)
        let mut old_mapping = create_test_mapping(111, -1008888888888);
        old_mapping.updated_at = now - 7200; // 2 hours ago
        store.save_mapping(&old_mapping).await.unwrap();

        // Create recent mapping (30 seconds ago)
        let mut recent_mapping = create_test_mapping(222, -1008888888888);
        recent_mapping.updated_at = now - 30;
        store.save_mapping(&recent_mapping).await.unwrap();

        // Query for mappings older than 1 hour
        let stale = store
            .get_stale_mappings(Duration::from_secs(3600))
            .await
            .unwrap();

        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].topic_id, 111);
    }

    #[tokio::test]
    async fn test_get_stale_mappings_returns_empty_when_none_stale() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let mapping = create_test_mapping(333, -1009999999999);
        store.save_mapping(&mapping).await.unwrap();

        // Query for mappings older than 1 hour (none should match)
        let stale = store
            .get_stale_mappings(Duration::from_secs(3600))
            .await
            .unwrap();
        assert_eq!(stale.len(), 0);
    }

    #[tokio::test]
    async fn test_mappings_persist_across_reconnects() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");

        // First connection
        {
            let store = TopicStore::new(&db_path).await.unwrap();
            let mapping = create_test_mapping(444, -1001010101010);
            store.save_mapping(&mapping).await.unwrap();
        }

        // Second connection
        {
            let store = TopicStore::new(&db_path).await.unwrap();
            let retrieved = store.get_mapping(444).await.unwrap();
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().topic_id, 444);
        }
    }

    #[tokio::test]
    async fn test_boolean_fields_handled_correctly() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");
        let store = TopicStore::new(&db_path).await.unwrap();

        let mut mapping = create_test_mapping(555, -1001111111111);
        mapping.streaming_enabled = false;
        mapping.topic_name_updated = true;
        store.save_mapping(&mapping).await.unwrap();

        let retrieved = store.get_mapping(555).await.unwrap().unwrap();
        assert_eq!(retrieved.streaming_enabled, false);
        assert_eq!(retrieved.topic_name_updated, true);
    }
}
