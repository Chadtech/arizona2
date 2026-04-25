-- person-task-state-column

BEGIN;

ALTER TABLE person_task
    ADD COLUMN IF NOT EXISTS state TEXT;

COMMIT;
