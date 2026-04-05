-- person-current-person-task

BEGIN;

ALTER TABLE person
    ADD COLUMN IF NOT EXISTS current_person_task_uuid UUID;

DO
$$
    BEGIN
        IF NOT EXISTS (SELECT 1
                       FROM pg_constraint
                       WHERE conname = 'person_fk_current_person_task') THEN
            ALTER TABLE person
                ADD CONSTRAINT person_fk_current_person_task
                    FOREIGN KEY (current_person_task_uuid)
                        REFERENCES person_task (uuid)
                        ON DELETE SET NULL;
        END IF;
    END
$$;

CREATE INDEX IF NOT EXISTS idx_person_current_person_task_uuid
    ON person (current_person_task_uuid);

COMMIT;
