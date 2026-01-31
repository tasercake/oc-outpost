pub mod log_store;
pub mod tracing_layer;

use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use std::path::Path;

/// Initialize the orchestrator database with instances table
pub async fn init_orchestrator_db(db_path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let url = format!("sqlite:{}?mode=rwc", db_path.display());
    let pool = SqlitePool::connect(&url).await?;

    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(&pool)
        .await?;

    let migration = include_str!("../../migrations/001_create_instances_table.sql");
    sqlx::query(migration).execute(&pool).await?;

    Ok(pool)
}

pub async fn init_log_db(db_path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let url = format!("sqlite:{}?mode=rwc", db_path.display());
    let pool = SqlitePool::connect(&url).await?;

    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(&pool)
        .await?;

    let migration = include_str!("../../migrations/003_create_log_tables.sql");
    sqlx::raw_sql(migration).execute(&pool).await?;

    // Add sequence column for log ordering (idempotent for existing DBs).
    // tokio::spawn in DatabaseLayer causes insertion order to differ from
    // emission order; this monotonic counter preserves the true order.
    let _ = sqlx::query("ALTER TABLE run_logs ADD COLUMN sequence INTEGER DEFAULT 0")
        .execute(&pool)
        .await;

    Ok(pool)
}

pub async fn init_topics_db(db_path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let url = format!("sqlite:{}?mode=rwc", db_path.display());
    let pool = SqlitePool::connect(&url).await?;

    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(&pool)
        .await?;

    let migration = include_str!("../../migrations/002_create_topic_mappings_table.sql");
    sqlx::query(migration).execute(&pool).await?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_init_orchestrator_db_creates_database() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("orchestrator.db");

        let pool = init_orchestrator_db(&db_path).await.unwrap();

        // Verify database file was created
        assert!(db_path.exists());

        // Verify we can query the database
        let result = sqlx::query("SELECT 1").fetch_one(&pool).await;
        assert!(result.is_ok());

        pool.close().await;
    }

    #[tokio::test]
    async fn test_init_orchestrator_db_creates_instances_table() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("orchestrator.db");

        let pool = init_orchestrator_db(&db_path).await.unwrap();

        // Verify instances table exists with correct schema
        let result = sqlx::query(
            "SELECT id, project_path, port, state, instance_type, session_id, created_at, updated_at 
             FROM instances LIMIT 0"
        )
        .fetch_optional(&pool)
        .await;

        assert!(
            result.is_ok(),
            "instances table should exist with correct columns"
        );

        pool.close().await;
    }

    #[tokio::test]
    async fn test_init_orchestrator_db_creates_indexes() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("orchestrator.db");

        let pool = init_orchestrator_db(&db_path).await.unwrap();

        // Verify indexes exist
        let indexes: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='instances'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        let index_names: Vec<String> = indexes.into_iter().map(|(name,)| name).collect();

        assert!(index_names.contains(&"idx_instances_port".to_string()));
        assert!(index_names.contains(&"idx_instances_project_path".to_string()));
        assert!(index_names.contains(&"idx_instances_state".to_string()));

        pool.close().await;
    }

    #[tokio::test]
    async fn test_init_orchestrator_db_is_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("orchestrator.db");

        // First initialization
        let pool1 = init_orchestrator_db(&db_path).await.unwrap();
        pool1.close().await;

        // Second initialization should not error
        let pool2 = init_orchestrator_db(&db_path).await.unwrap();

        // Verify table still exists and is functional
        let result = sqlx::query("SELECT COUNT(*) FROM instances")
            .fetch_one(&pool2)
            .await;
        assert!(result.is_ok());

        pool2.close().await;
    }

    #[tokio::test]
    async fn test_init_orchestrator_db_enables_wal_mode() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("orchestrator.db");

        let pool = init_orchestrator_db(&db_path).await.unwrap();

        // Verify WAL mode is enabled
        let (journal_mode,): (String,) = sqlx::query_as("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(journal_mode.to_lowercase(), "wal");

        pool.close().await;
    }

    #[tokio::test]
    async fn test_init_topics_db_creates_database() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");

        let pool = init_topics_db(&db_path).await.unwrap();

        // Verify database file was created
        assert!(db_path.exists());

        // Verify we can query the database
        let result = sqlx::query("SELECT 1").fetch_one(&pool).await;
        assert!(result.is_ok());

        pool.close().await;
    }

    #[tokio::test]
    async fn test_init_topics_db_creates_topic_mappings_table() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");

        let pool = init_topics_db(&db_path).await.unwrap();

        // Verify topic_mappings table exists with correct schema
        let result = sqlx::query(
            "SELECT topic_id, chat_id, project_path, session_id, instance_id, 
                    streaming_enabled, topic_name_updated, created_at, updated_at 
             FROM topic_mappings LIMIT 0",
        )
        .fetch_optional(&pool)
        .await;

        assert!(
            result.is_ok(),
            "topic_mappings table should exist with correct columns"
        );

        pool.close().await;
    }

    #[tokio::test]
    async fn test_init_topics_db_creates_indexes() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");

        let pool = init_topics_db(&db_path).await.unwrap();

        // Verify indexes exist
        let indexes: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='topic_mappings'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        let index_names: Vec<String> = indexes.into_iter().map(|(name,)| name).collect();

        assert!(index_names.contains(&"idx_topic_mappings_chat_id".to_string()));
        assert!(index_names.contains(&"idx_topic_mappings_session_id".to_string()));
        assert!(index_names.contains(&"idx_topic_mappings_instance_id".to_string()));

        pool.close().await;
    }

    #[tokio::test]
    async fn test_init_topics_db_is_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");

        // First initialization
        let pool1 = init_topics_db(&db_path).await.unwrap();
        pool1.close().await;

        // Second initialization should not error
        let pool2 = init_topics_db(&db_path).await.unwrap();

        // Verify table still exists and is functional
        let result = sqlx::query("SELECT COUNT(*) FROM topic_mappings")
            .fetch_one(&pool2)
            .await;
        assert!(result.is_ok());

        pool2.close().await;
    }

    #[tokio::test]
    async fn test_init_topics_db_enables_wal_mode() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");

        let pool = init_topics_db(&db_path).await.unwrap();

        // Verify WAL mode is enabled
        let (journal_mode,): (String,) = sqlx::query_as("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(journal_mode.to_lowercase(), "wal");

        pool.close().await;
    }

    #[tokio::test]
    async fn test_init_topics_db_default_values() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("topics.db");

        let pool = init_topics_db(&db_path).await.unwrap();

        // Insert a minimal record to test defaults
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        sqlx::query(
            "INSERT INTO topic_mappings (topic_id, chat_id, project_path, created_at, updated_at) 
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(12345)
        .bind(67890)
        .bind("/test/path")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

        // Verify defaults were applied
        let (streaming_enabled, topic_name_updated): (i32, i32) = sqlx::query_as(
            "SELECT streaming_enabled, topic_name_updated FROM topic_mappings WHERE topic_id = ?",
        )
        .bind(12345)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(
            streaming_enabled, 1,
            "streaming_enabled should default to 1"
        );
        assert_eq!(
            topic_name_updated, 0,
            "topic_name_updated should default to 0"
        );

        pool.close().await;
    }
}
