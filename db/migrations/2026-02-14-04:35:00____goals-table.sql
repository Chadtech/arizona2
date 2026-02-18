-- goals-table

BEGIN;

CREATE TABLE IF NOT EXISTS goal
(
    uuid         UUID PRIMARY KEY,
    person_uuid  UUID        NOT NULL,
    content      TEXT        NOT NULL,
    priority     INTEGER     NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    ended_at     TIMESTAMPTZ,
    deleted_at   TIMESTAMPTZ
);

DO
$$
    BEGIN
        IF NOT EXISTS (SELECT 1
                       FROM pg_constraint
                       WHERE conname = 'goal_fk_person') THEN
            ALTER TABLE goal
                ADD CONSTRAINT goal_fk_person
                    FOREIGN KEY (person_uuid)
                        REFERENCES person (uuid)
                        ON DELETE CASCADE;
        END IF;
    END
$$;

CREATE INDEX IF NOT EXISTS idx_goal_person ON goal (person_uuid);
CREATE INDEX IF NOT EXISTS idx_goal_priority ON goal (priority);
CREATE INDEX IF NOT EXISTS idx_goal_ended_at ON goal (ended_at);
CREATE INDEX IF NOT EXISTS idx_goal_deleted_at ON goal (deleted_at);

COMMIT;
