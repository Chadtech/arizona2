-- person-table

BEGIN;

CREATE TABLE IF NOT EXISTS person
(
    uuid       UUID PRIMARY KEY NOT NULL,
    name       TEXT             NOT NULL,
    created_at TIMESTAMP        NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP
);

COMMIT;