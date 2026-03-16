-- memory-v2-columns

BEGIN;

ALTER TABLE memory
    ADD COLUMN IF NOT EXISTS summary              TEXT,
    ADD COLUMN IF NOT EXISTS retrieval_summary    TEXT,
    ADD COLUMN IF NOT EXISTS summary_first_person TEXT,
    ADD COLUMN IF NOT EXISTS emotional_score      INTEGER,
    ADD COLUMN IF NOT EXISTS people_names         TEXT[],
    ADD COLUMN IF NOT EXISTS people_uuids         UUID[],
    ADD COLUMN IF NOT EXISTS subject_tags         TEXT[],
    ADD COLUMN IF NOT EXISTS in_world_time        TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS is_v2                BOOLEAN NOT NULL DEFAULT FALSE;

COMMIT;
