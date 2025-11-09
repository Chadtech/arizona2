-- allow-null-message-sender
-- Allow NULL sender_person_uuid to represent messages from real world users (e.g., Chad)

BEGIN;

-- Drop the NOT NULL constraint on sender_person_uuid
ALTER TABLE message ALTER COLUMN sender_person_uuid DROP NOT NULL;

COMMIT;
