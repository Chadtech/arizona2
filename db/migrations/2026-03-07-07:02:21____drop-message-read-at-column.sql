-- drop-message-read-at-column

BEGIN;
ALTER TABLE message
    DROP COLUMN IF EXISTS read_at;
COMMIT;
