-- person-task-table

BEGIN;

CREATE TABLE IF NOT EXISTS person_task
(
    uuid               UUID PRIMARY KEY,
    person_uuid        UUID        NOT NULL,
    content            TEXT        NOT NULL,
    success_condition  TEXT,
    abandon_condition  TEXT,
    failure_condition  TEXT,
    priority           INTEGER     NOT NULL CHECK (priority >= 0 AND priority <= 100),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at       TIMESTAMPTZ,
    abandoned_at       TIMESTAMPTZ,
    failed_at          TIMESTAMPTZ
);

DO
$$
    BEGIN
        IF NOT EXISTS (SELECT 1
                       FROM pg_constraint
                       WHERE conname = 'person_task_fk_person') THEN
            ALTER TABLE person_task
                ADD CONSTRAINT person_task_fk_person
                    FOREIGN KEY (person_uuid)
                        REFERENCES person (uuid)
                        ON DELETE CASCADE;
        END IF;
    END
$$;

CREATE INDEX IF NOT EXISTS idx_person_task_person ON person_task (person_uuid);
CREATE INDEX IF NOT EXISTS idx_person_task_priority ON person_task (priority);
CREATE INDEX IF NOT EXISTS idx_person_task_completed_at ON person_task (completed_at);
CREATE INDEX IF NOT EXISTS idx_person_task_abandoned_at ON person_task (abandoned_at);
CREATE INDEX IF NOT EXISTS idx_person_task_failed_at ON person_task (failed_at);

COMMIT;
