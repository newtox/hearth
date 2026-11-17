ALTER TABLE users ADD COLUMN email_verified BOOLEAN NOT NULL DEFAULT false;
UPDATE users SET email_verified = true;
CREATE TABLE email_verifications (
    token_hash TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);
