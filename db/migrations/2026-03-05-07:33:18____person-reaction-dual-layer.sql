-- person-reaction-dual-layer

BEGIN;

ALTER TABLE person
    ADD COLUMN IF NOT EXISTS reaction_dual_layer BOOLEAN NOT NULL DEFAULT FALSE;

COMMIT;
