-- person-task-historical-state

BEGIN;

CREATE TABLE IF NOT EXISTS person_task_historical_state
(
    uuid             UUID PRIMARY KEY,
    person_task_uuid UUID        NOT NULL,
    content          TEXT        NOT NULL,
    state_before     TEXT,
    changed_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

DO
$$
    BEGIN
        IF NOT EXISTS (SELECT 1
                       FROM pg_constraint
                       WHERE conname = 'person_task_historical_state_fk_person_task') THEN
            ALTER TABLE person_task_historical_state
                ADD CONSTRAINT person_task_historical_state_fk_person_task
                    FOREIGN KEY (person_task_uuid)
                        REFERENCES person_task (uuid)
                        ON DELETE CASCADE;
        END IF;
    END
$$;

CREATE INDEX IF NOT EXISTS idx_person_task_historical_state_person_task
    ON person_task_historical_state (person_task_uuid);

CREATE INDEX IF NOT EXISTS idx_person_task_historical_state_changed_at
    ON person_task_historical_state (changed_at);

COMMIT;
