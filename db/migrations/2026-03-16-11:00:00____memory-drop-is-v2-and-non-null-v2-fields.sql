-- memory-drop-is-v2-and-non-null-v2-fields

BEGIN;

-- Backfill nullable v2 fields so NOT NULL constraints can be applied safely.
UPDATE memory
SET
    summary = COALESCE(summary, content),
    retrieval_summary = COALESCE(retrieval_summary, summary, content),
    summary_first_person = COALESCE(summary_first_person, summary, content),
    emotional_score = COALESCE(emotional_score, 50),
    people_names = COALESCE(people_names, ARRAY[]::TEXT[]),
    people_uuids = COALESCE(people_uuids, ARRAY[]::UUID[]),
    subject_tags = COALESCE(subject_tags, ARRAY[]::TEXT[]);

ALTER TABLE memory
    ALTER COLUMN summary SET NOT NULL,
    ALTER COLUMN retrieval_summary SET NOT NULL,
    ALTER COLUMN summary_first_person SET NOT NULL,
    ALTER COLUMN emotional_score SET NOT NULL,
    ALTER COLUMN people_names SET NOT NULL,
    ALTER COLUMN people_uuids SET NOT NULL,
    ALTER COLUMN subject_tags SET NOT NULL,
    DROP COLUMN IF EXISTS is_v2;

COMMIT;
