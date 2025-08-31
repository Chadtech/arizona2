-- state-of-mind-table

BEGIN;

CREATE TABLE IF NOT EXISTS state_of_mind
(
    uuid        UUID PRIMARY KEY,
    person_uuid UUID        NOT NULL,
    content     TEXT        NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Ensure foreign key constraint to person exists
DO
$$
    BEGIN
        IF NOT EXISTS (SELECT 1
                       FROM pg_constraint
                       WHERE conname = 'state_of_mind_fk_person') THEN
            ALTER TABLE state_of_mind
                ADD CONSTRAINT state_of_mind_fk_person
                FOREIGN KEY (person_uuid)
                REFERENCES person (uuid)
                ON DELETE CASCADE;
        END IF;
    END
$$;

COMMIT;