-- scene-participant-indexes

BEGIN;

CREATE INDEX IF NOT EXISTS idx_scene_participant_active_scene
    ON scene_participant (scene_uuid, joined_at DESC)
    INCLUDE (person_uuid)
    WHERE left_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_scene_participant_active_person
    ON scene_participant (person_uuid, joined_at DESC)
    INCLUDE (scene_uuid)
    WHERE left_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_scene_participant_scene_history
    ON scene_participant (scene_uuid, joined_at ASC);

CREATE INDEX IF NOT EXISTS idx_scene_participant_person_history
    ON scene_participant (person_uuid, joined_at DESC);

COMMIT;
