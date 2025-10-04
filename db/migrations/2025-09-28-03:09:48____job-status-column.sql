-- job-status-column

BEGIN;

ALTER TABLE job
    ADD COLUMN IF NOT EXISTS started_at TIMESTAMP;

COMMIT;