ALTER TABLE messages ADD COLUMN kind TEXT NOT NULL DEFAULT 'default';
ALTER TABLE messages ADD COLUMN call_ended_at TIMESTAMPTZ;
ALTER TABLE messages ADD COLUMN call_participants UUID[] NOT NULL DEFAULT '{}';
