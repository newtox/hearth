CREATE TABLE user_stats (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    play_seconds BIGINT NOT NULL DEFAULT 0,
    stream_seconds BIGINT NOT NULL DEFAULT 0,
    voice_seconds BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE spotify_links (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    refresh_token TEXT NOT NULL,
    access_token TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE spotify_states (
    state TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
