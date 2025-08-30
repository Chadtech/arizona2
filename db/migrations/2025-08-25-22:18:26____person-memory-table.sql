-- person-memory-table

BEGIN;

CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS memory
(
    uuid        UUID PRIMARY KEY,
    person_uuid UUID         NOT NULL,
    content     TEXT         NOT NULL,
    -- vector for semantic search
    embedding   vector(1536) NOT NULL,
    -- useful for time decay / recency
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT now()
);

-- Ensure foreign key constraint to person exists
DO
$$
    BEGIN
        IF NOT EXISTS (SELECT 1
                       FROM pg_constraint
                       WHERE conname = 'memory_fk_person') THEN
            ALTER TABLE memory
                ADD CONSTRAINT memory_fk_person
                FOREIGN KEY (person_uuid)
                REFERENCES person (uuid)
                ON DELETE CASCADE;
        END IF;
    END
$$;

COMMIT;