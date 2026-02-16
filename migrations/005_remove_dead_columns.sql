-- streaming is always on
ALTER TABLE topic_mappings DROP COLUMN streaming_enabled;

-- all instances are now managed
ALTER TABLE instances DROP COLUMN instance_type;
