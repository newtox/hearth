ALTER TABLE roles ADD COLUMN mentionable BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE messages ADD COLUMN mention_everyone BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE messages ADD COLUMN mention_roles UUID[] NOT NULL DEFAULT '{}';
UPDATE messages m SET mention_everyone = true FROM channels c
WHERE c.id = m.channel_id AND c.guild_id IS NOT NULL AND m.content ~* '(^|[^a-z0-9_@])@(everyone|here)([^a-z0-9_.]|$)';
CREATE TABLE notify_prefs (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_id UUID NOT NULL,
    level TEXT,
    muted BOOLEAN NOT NULL DEFAULT false,
    suppress_everyone BOOLEAN NOT NULL DEFAULT false,
    suppress_roles BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (user_id, target_id)
);
