CREATE TABLE IF NOT EXISTS topic_mappings_new (
    chat_id INTEGER NOT NULL,
    topic_id INTEGER NOT NULL,
    project_path TEXT NOT NULL,
    session_id TEXT,
    instance_id TEXT,
    topic_name_updated INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (chat_id, topic_id)
);

INSERT OR IGNORE INTO topic_mappings_new
    SELECT chat_id, topic_id, project_path, session_id, instance_id,
           topic_name_updated, created_at, updated_at
    FROM topic_mappings;

DROP TABLE topic_mappings;

ALTER TABLE topic_mappings_new RENAME TO topic_mappings;

CREATE INDEX IF NOT EXISTS idx_topic_mappings_chat_id ON topic_mappings(chat_id);
CREATE INDEX IF NOT EXISTS idx_topic_mappings_session_id ON topic_mappings(session_id);
CREATE INDEX IF NOT EXISTS idx_topic_mappings_instance_id ON topic_mappings(instance_id);
