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


COMMIT;