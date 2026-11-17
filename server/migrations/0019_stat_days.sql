CREATE TABLE user_stat_days (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    day DATE NOT NULL,
    play_seconds BIGINT NOT NULL DEFAULT 0,
    stream_seconds BIGINT NOT NULL DEFAULT 0,
    voice_seconds BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, day)
);
