use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

use super::init_log_db;

#[derive(Clone)]
pub struct LogStore {
    pool: SqlitePool,
}

impl LogStore {
    pub async fn new(db_path: &Path) -> Result<Self> {
        let pool = init_log_db(db_path).await?;
        // NOTE: This log may not appear at startup since tracing isn't initialized yet.
        debug!(db_path = %db_path.display(), "Log store initialized");
        Ok(Self { pool })
    }

    pub async fn create_run(
        &self,
        run_id: &str,
        version: &str,
        config_summary: Option<&str>,
    ) -> Result<()> {
        debug!(run_id = %run_id, version = %version, "Creating run record");
        let now = now_millis();

        sqlx::query(
            "INSERT INTO bot_runs (run_id, started_at, version, config_summary)
             VALUES (?, ?, ?, ?)",
        )
        .bind(run_id)
        .bind(now)
        .bind(version)
        .bind(config_summary)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn finish_run(&self, run_id: &str) -> Result<()> {
        debug!(run_id = %run_id, "Finishing run record");
        let now = now_millis();

        sqlx::query("UPDATE bot_runs SET stopped_at = ? WHERE run_id = ?")
            .bind(now)
            .bind(run_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // NOTE: Do NOT add tracing/logging calls to this method.
    // This method is called by DatabaseLayer::on_event() for every log event.
    // Adding a log call here would cause infinite recursion.
    pub async fn insert_log(
        &self,
        run_id: &str,
        timestamp: i64,
        sequence: i64,
        level: &str,
        target: &str,
        message: &str,
        fields: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO run_logs (run_id, timestamp, sequence, level, target, message, fields)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(run_id)
        .bind(timestamp)
        .bind(sequence)
        .bind(level)
        .bind(target)
        .bind(message)
        .bind(fields)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[cfg(test)]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_and_finish_run() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("logs.db");
        let store = LogStore::new(&db_path).await.unwrap();

        store
            .create_run("run_test123", "0.1.0", Some(r#"{"max_instances":10}"#))
            .await
            .unwrap();

        let (run_id, version): (String, String) =
            sqlx::query_as("SELECT run_id, version FROM bot_runs WHERE run_id = ?")
                .bind("run_test123")
                .fetch_one(store.pool())
                .await
                .unwrap();
        assert_eq!(run_id, "run_test123");
        assert_eq!(version, "0.1.0");

        store.finish_run("run_test123").await.unwrap();

        let (stopped_at,): (Option<i64>,) =
            sqlx::query_as("SELECT stopped_at FROM bot_runs WHERE run_id = ?")
                .bind("run_test123")
                .fetch_one(store.pool())
                .await
                .unwrap();
        assert!(stopped_at.is_some());
    }

    #[tokio::test]
    async fn test_insert_and_query_logs() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("logs.db");
        let store = LogStore::new(&db_path).await.unwrap();

        store.create_run("run_abc", "0.1.0", None).await.unwrap();

        store
            .insert_log(
                "run_abc",
                1000,
                0,
                "INFO",
                "oc_outpost::bot",
                "Bot started",
                None,
            )
            .await
            .unwrap();

        store
            .insert_log(
                "run_abc",
                2000,
                1,
                "ERROR",
                "oc_outpost::orchestrator",
                "Instance crashed",
                Some(r#"{"instance_id":"inst-1"}"#),
            )
            .await
            .unwrap();

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM run_logs WHERE run_id = ?")
            .bind("run_abc")
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert_eq!(count, 2);

        let (level,): (String,) = sqlx::query_as(
            "SELECT level FROM run_logs WHERE run_id = ? ORDER BY timestamp DESC LIMIT 1",
        )
        .bind("run_abc")
        .fetch_one(store.pool())
        .await
        .unwrap();
        assert_eq!(level, "ERROR");
    }
}
