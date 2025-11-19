-- drop-message-type-column

BEGIN;

ALTER TABLE message DROP COLUMN IF EXISTS message_type;

COMMIT;