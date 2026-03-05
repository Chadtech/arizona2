-- person-scene-visit-table

BEGIN;

CREATE TABLE IF NOT EXISTS person_scene_visit
(
    person_uuid      UUID        NOT NULL REFERENCES person (uuid) ON DELETE CASCADE,
    scene_uuid       UUID        NOT NULL REFERENCES scene (uuid) ON DELETE CASCADE,
    first_visited_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_visited_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    visit_count      INTEGER     NOT NULL DEFAULT 1,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ,
    PRIMARY KEY (person_uuid, scene_uuid)
);

INSERT INTO person_scene_visit (
    person_uuid,
    scene_uuid,
    first_visited_at,
    last_visited_at,
    visit_count,
    created_at,
    updated_at
)
SELECT scene_participant.person_uuid,
       scene_participant.scene_uuid,
       MIN(scene_participant.joined_at) AS first_visited_at,
       MAX(scene_participant.joined_at) AS last_visited_at,
       COUNT(*)::INTEGER                AS visit_count,
       now()                            AS created_at,
       now()                            AS updated_at
FROM scene_participant
GROUP BY scene_participant.person_uuid, scene_participant.scene_uuid
ON CONFLICT (person_uuid, scene_uuid) DO NOTHING;

COMMIT;
