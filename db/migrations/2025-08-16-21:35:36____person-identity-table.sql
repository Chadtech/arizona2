-- person-identity-table

BEGIN;

-- Create person_identity table with its own id, a reference to person id,
-- a created_at timestamp, and a text field for the identity itself
CREATE TABLE IF NOT EXISTS person_identity
(
    uuid        UUID PRIMARY KEY NOT NULL,
    person_uuid UUID             NOT NULL,
    identity    TEXT             NOT NULL,
    created_at  TIMESTAMP        NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_person
        FOREIGN KEY (person_uuid)
            REFERENCES person (uuid)
            ON DELETE CASCADE
);

-- Add a unique constraint on the person table to ensure all names are unique
DO
$$
    BEGIN
        -- Check if the constraint already exists
        IF NOT EXISTS (SELECT 1
                       FROM pg_constraint
                       WHERE conname = 'unique_person_name') THEN
            -- Add the constraint only if it doesn't exist
            ALTER TABLE person
                ADD CONSTRAINT unique_person_name UNIQUE (name);
        END IF;
    END
$$;

COMMIT;