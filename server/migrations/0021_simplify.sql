DROP TABLE IF EXISTS spotify_links;
DROP TABLE IF EXISTS spotify_states;
DROP TABLE IF EXISTS connections;
DROP TABLE IF EXISTS oauth_states;
ALTER TABLE user_stats DROP COLUMN IF EXISTS play_seconds;
ALTER TABLE user_stat_days DROP COLUMN IF EXISTS play_seconds;
