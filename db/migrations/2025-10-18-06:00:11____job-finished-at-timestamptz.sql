-- job-finished-at-timestamptz

BEGIN;

ALTER TABLE job
    ALTER COLUMN created_at DROP DEFAULT,
    ALTER COLUMN created_at TYPE timestamptz
        USING (created_at AT TIME ZONE 'UTC'),

    ALTER COLUMN created_at SET DEFAULT NOW(),
    ALTER COLUMN created_at SET NOT NULL,

    ALTER COLUMN finished_at DROP DEFAULT,
    ALTER COLUMN finished_at TYPE timestamptz
        USING (finished_at AT TIME ZONE 'UTC'),

    ALTER COLUMN started_at DROP DEFAULT,
    ALTER COLUMN started_at TYPE timestamptz
        USING (started_at AT TIME ZONE 'UTC');


COMMIT;