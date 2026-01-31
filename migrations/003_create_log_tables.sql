-- Bot run tracking and log storage
-- Each bot process run gets a unique run_id for log grouping

CREATE TABLE IF NOT EXISTS bot_runs (
    run_id TEXT PRIMARY KEY,
    started_at INTEGER NOT NULL,
    stopped_at INTEGER,
    version TEXT NOT NULL,
    config_summary TEXT
);

CREATE TABLE IF NOT EXISTS run_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL REFERENCES bot_runs(run_id),
    timestamp INTEGER NOT NULL,
    level TEXT NOT NULL,
    target TEXT NOT NULL,
    message TEXT NOT NULL,
    fields TEXT
);

CREATE INDEX IF NOT EXISTS idx_run_logs_run_id ON run_logs(run_id);
CREATE INDEX IF NOT EXISTS idx_run_logs_level ON run_logs(level);
CREATE INDEX IF NOT EXISTS idx_run_logs_timestamp ON run_logs(timestamp);
