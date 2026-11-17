DELETE FROM overwrites WHERE target_id IN (SELECT id FROM roles WHERE managed IS NOT NULL);
DROP TABLE integrations;
DELETE FROM roles WHERE managed IS NOT NULL;
ALTER TABLE roles DROP COLUMN managed;
DELETE FROM connections WHERE provider <> 'spotify';
DROP TABLE oauth_states;
