-- drop-person-reaction-dual-layer

BEGIN;

ALTER TABLE person
    DROP COLUMN IF EXISTS reaction_dual_layer;

COMMIT;
