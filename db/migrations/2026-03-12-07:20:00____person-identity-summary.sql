-- person-identity-summary

BEGIN;

ALTER TABLE person_identity
    ADD COLUMN IF NOT EXISTS summary TEXT;

COMMIT;
