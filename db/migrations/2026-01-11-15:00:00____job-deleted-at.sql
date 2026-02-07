-- job-deleted-at

BEGIN;

ALTER TABLE job
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;

COMMIT;
