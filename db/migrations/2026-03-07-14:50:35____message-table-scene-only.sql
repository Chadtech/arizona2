-- message-table-scene-only

BEGIN;

-- Remove rows that do not fit the scene-only message model.
DELETE
FROM message
WHERE scene_uuid IS NULL;

-- Enforce scene-only constraints.
ALTER TABLE message
    ALTER COLUMN scene_uuid SET NOT NULL;

ALTER TABLE message
    DROP COLUMN IF EXISTS receiver_person_uuid;

COMMIT;
