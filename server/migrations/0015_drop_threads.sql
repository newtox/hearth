DELETE FROM channels WHERE kind = 'thread';
ALTER TABLE channels DROP COLUMN starter_id;
