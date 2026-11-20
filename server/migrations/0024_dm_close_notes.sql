ALTER TABLE dm_recipients ADD COLUMN hidden BOOLEAN NOT NULL DEFAULT false;

CREATE TABLE user_notes (
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    note TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (owner_id, target_id)
);
