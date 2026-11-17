CREATE TABLE polls (
    message_id UUID PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
    question TEXT NOT NULL,
    multi BOOLEAN NOT NULL DEFAULT false,
    expires_at TIMESTAMPTZ
);

CREATE TABLE poll_options (
    id UUID PRIMARY KEY,
    message_id UUID NOT NULL REFERENCES polls(message_id) ON DELETE CASCADE,
    position INT NOT NULL,
    text TEXT NOT NULL
);
CREATE INDEX poll_options_message ON poll_options(message_id);

CREATE TABLE poll_votes (
    option_id UUID NOT NULL REFERENCES poll_options(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    message_id UUID NOT NULL,
    PRIMARY KEY (option_id, user_id)
);
CREATE INDEX poll_votes_message ON poll_votes(message_id, user_id);
