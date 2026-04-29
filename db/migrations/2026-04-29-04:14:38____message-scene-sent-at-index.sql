-- message-scene-sent-at-index

BEGIN;

CREATE INDEX IF NOT EXISTS idx_message_scene_sent_at_desc
    ON message (scene_uuid, sent_at DESC);

COMMIT;
