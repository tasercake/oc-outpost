-- Orchestrator instances table
-- Tracks all OpenCode instances managed by the orchestrator
CREATE TABLE IF NOT EXISTS instances (
    id TEXT PRIMARY KEY,                -- Unique instance identifier
    project_path TEXT NOT NULL,         -- Absolute path to project directory
    port INTEGER NOT NULL,              -- Port number where instance is running
    state TEXT NOT NULL,                -- Instance state: starting, running, stopping, stopped, error
    session_id TEXT,                    -- OpenCode session ID (if available)
    created_at INTEGER NOT NULL,        -- Unix timestamp when instance was created
    updated_at INTEGER NOT NULL         -- Unix timestamp when instance was last updated
);

-- Performance indexes for common queries
CREATE INDEX IF NOT EXISTS idx_instances_port ON instances(port);
CREATE INDEX IF NOT EXISTS idx_instances_project_path ON instances(project_path);
CREATE INDEX IF NOT EXISTS idx_instances_state ON instances(state);
