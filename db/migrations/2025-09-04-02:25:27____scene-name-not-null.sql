-- scene-constraints

BEGIN;

ALTER TABLE scene
    ALTER COLUMN name SET NOT NULL;

COMMIT;