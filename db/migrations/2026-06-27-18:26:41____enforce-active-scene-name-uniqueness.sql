-- enforce-active-scene-name-uniqueness

BEGIN;

ALTER TABLE scene
    DROP CONSTRAINT IF EXISTS unique_scene_name;

DROP INDEX IF EXISTS scene_active_normalized_name_unique;

UPDATE scene
SET name = format('Untitled scene %s', uuid)
WHERE btrim(name) = '';

CREATE TEMP TABLE active_scene_name_duplicate_map ON COMMIT DROP AS
WITH active_scenes AS (
    SELECT
        uuid,
        started_at,
        regexp_replace(btrim(name), '\s+', ' ', 'g') AS normalized_name,
        lower(regexp_replace(btrim(name), '\s+', ' ', 'g')) AS normalized_key
    FROM scene
    WHERE ended_at IS NULL
),
ranked AS (
    SELECT
        uuid,
        first_value(uuid) OVER (
            PARTITION BY normalized_key
            ORDER BY started_at ASC, uuid ASC
        ) AS canonical_uuid,
        first_value(normalized_name) OVER (
            PARTITION BY normalized_key
            ORDER BY started_at ASC, uuid ASC
        ) AS canonical_name
    FROM active_scenes
)
SELECT
    uuid AS duplicate_uuid,
    canonical_uuid,
    canonical_name
FROM ranked
WHERE uuid <> canonical_uuid;

INSERT INTO person_scene_visit (
    person_uuid,
    scene_uuid,
    first_visited_at,
    last_visited_at,
    visit_count,
    created_at,
    updated_at
)
SELECT
    person_scene_visit.person_uuid,
    active_scene_name_duplicate_map.canonical_uuid,
    MIN(person_scene_visit.first_visited_at),
    MAX(person_scene_visit.last_visited_at),
    SUM(person_scene_visit.visit_count)::INTEGER,
    MIN(person_scene_visit.created_at),
    NOW()
FROM person_scene_visit
JOIN active_scene_name_duplicate_map
    ON person_scene_visit.scene_uuid = active_scene_name_duplicate_map.duplicate_uuid
GROUP BY
    person_scene_visit.person_uuid,
    active_scene_name_duplicate_map.canonical_uuid
ON CONFLICT (person_uuid, scene_uuid)
DO UPDATE
SET first_visited_at = LEAST(person_scene_visit.first_visited_at, EXCLUDED.first_visited_at),
    last_visited_at = GREATEST(person_scene_visit.last_visited_at, EXCLUDED.last_visited_at),
    visit_count = person_scene_visit.visit_count + EXCLUDED.visit_count,
    updated_at = NOW();

DELETE FROM person_scene_visit
USING active_scene_name_duplicate_map
WHERE person_scene_visit.scene_uuid = active_scene_name_duplicate_map.duplicate_uuid;

INSERT INTO real_world_user_scene_presence (scene_uuid)
SELECT DISTINCT canonical_uuid
FROM active_scene_name_duplicate_map
JOIN real_world_user_scene_presence
    ON real_world_user_scene_presence.scene_uuid = active_scene_name_duplicate_map.duplicate_uuid
ON CONFLICT (scene_uuid) DO NOTHING;

DELETE FROM real_world_user_scene_presence
USING active_scene_name_duplicate_map
WHERE real_world_user_scene_presence.scene_uuid = active_scene_name_duplicate_map.duplicate_uuid;

UPDATE message
SET scene_uuid = active_scene_name_duplicate_map.canonical_uuid
FROM active_scene_name_duplicate_map
WHERE message.scene_uuid = active_scene_name_duplicate_map.duplicate_uuid;

UPDATE scene_participant
SET scene_uuid = active_scene_name_duplicate_map.canonical_uuid
FROM active_scene_name_duplicate_map
WHERE scene_participant.scene_uuid = active_scene_name_duplicate_map.duplicate_uuid;

UPDATE scene
SET name = active_scene_name_duplicate_map.canonical_name
FROM active_scene_name_duplicate_map
WHERE scene.uuid = active_scene_name_duplicate_map.canonical_uuid;

UPDATE scene
SET ended_at = COALESCE(ended_at, NOW())
FROM active_scene_name_duplicate_map
WHERE scene.uuid = active_scene_name_duplicate_map.duplicate_uuid;

UPDATE scene
SET name = regexp_replace(btrim(name), '\s+', ' ', 'g')
WHERE name <> regexp_replace(btrim(name), '\s+', ' ', 'g');

ALTER TABLE scene
    DROP CONSTRAINT IF EXISTS scene_name_not_blank;

ALTER TABLE scene
    ADD CONSTRAINT scene_name_not_blank CHECK (btrim(name) <> '');

CREATE UNIQUE INDEX IF NOT EXISTS scene_active_normalized_name_unique
    ON scene (lower(btrim(name)))
    WHERE ended_at IS NULL;

COMMIT;
