ALTER TABLE messages ADD COLUMN reply_to UUID;
ALTER TABLE messages ADD COLUMN pinned_at TIMESTAMPTZ;
CREATE INDEX messages_pinned ON messages(channel_id, pinned_at) WHERE pinned_at IS NOT NULL;

CREATE TABLE password_resets (
    token_hash TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL
);
