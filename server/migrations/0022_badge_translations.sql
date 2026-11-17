ALTER TABLE badges ADD COLUMN translations JSONB NOT NULL DEFAULT '{}'::jsonb;
