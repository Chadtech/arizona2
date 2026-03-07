-- drop-message-embedding-column

BEGIN;
ALTER TABLE message
    DROP COLUMN IF EXISTS embedding;
COMMIT;
