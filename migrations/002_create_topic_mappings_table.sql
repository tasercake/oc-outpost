-- Telegram forum topic mappings table
-- Maps Telegram forum topics to OpenCode instances and sessions
CREATE TABLE IF NOT EXISTS topic_mappings (
    topic_id INTEGER PRIMARY KEY,               -- Telegram forum topic ID
    chat_id INTEGER NOT NULL,                   -- Telegram chat ID (supergroup)
    project_path TEXT NOT NULL,                 -- Absolute path to project directory
    session_id TEXT,                            -- OpenCode session ID (if connected)
    instance_id TEXT,                           -- Associated instance ID (if any)
    streaming_enabled INTEGER NOT NULL DEFAULT 1,    -- 1 = streaming on, 0 = off
    topic_name_updated INTEGER NOT NULL DEFAULT 0,   -- 1 = topic name synced, 0 = not synced
    created_at INTEGER NOT NULL,                -- Unix timestamp when mapping was created
    updated_at INTEGER NOT NULL                 -- Unix timestamp when mapping was last updated
);

-- Performance indexes for common queries
CREATE INDEX IF NOT EXISTS idx_topic_mappings_chat_id ON topic_mappings(chat_id);
CREATE INDEX IF NOT EXISTS idx_topic_mappings_session_id ON topic_mappings(session_id);
CREATE INDEX IF NOT EXISTS idx_topic_mappings_instance_id ON topic_mappings(instance_id);
