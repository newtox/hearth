ALTER TABLE roles ADD COLUMN managed TEXT;
ALTER TABLE oauth_states ADD COLUMN guild_id UUID REFERENCES guilds(id) ON DELETE CASCADE;

CREATE TABLE integrations (
    id UUID PRIMARY KEY,
    guild_id UUID NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    account_id TEXT NOT NULL,
    account_name TEXT NOT NULL,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    access_token TEXT NOT NULL,
    refresh_token TEXT,
    expires_at TIMESTAMPTZ,
    synced_at TIMESTAMPTZ,
    error TEXT,
    UNIQUE (guild_id, provider)
);
