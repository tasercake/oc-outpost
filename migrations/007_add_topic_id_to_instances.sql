-- Add topic_id to instances table for OpenCode data directory isolation
ALTER TABLE instances ADD COLUMN topic_id INTEGER NOT NULL DEFAULT 0;

-- Create index for topic_id lookups
CREATE INDEX IF NOT EXISTS idx_instances_topic_id ON instances(topic_id);
