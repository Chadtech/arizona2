-- person-hibernation-column

BEGIN;

ALTER TABLE person
    ADD COLUMN IF NOT EXISTS is_hibernating BOOLEAN NOT NULL DEFAULT FALSE;

COMMIT;
