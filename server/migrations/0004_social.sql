CREATE TABLE friendships (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    friend_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, friend_id)
);
CREATE INDEX friendships_friend ON friendships(friend_id);

ALTER TABLE channels ADD COLUMN owner_id UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE channels ADD COLUMN icon TEXT;
ALTER TABLE channels ADD COLUMN settings JSONB NOT NULL DEFAULT '{}';
ALTER TABLE dm_recipients ADD COLUMN role SMALLINT NOT NULL DEFAULT 0;

CREATE TABLE blocks (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, blocked_id)
);
CREATE INDEX blocks_blocked ON blocks(blocked_id);
