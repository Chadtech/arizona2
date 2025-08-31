-- scene_tables

BEGIN;

CREATE TABLE IF NOT EXISTS scene
(
    uuid       UUID PRIMARY KEY,
    name       TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ended_at   TIMESTAMPTZ
);


DO
$$
    BEGIN
        IF NOT EXISTS (SELECT 1
                       FROM pg_constraint
                       WHERE conname = 'unique_scene_name') THEN
            ALTER TABLE scene
                ADD CONSTRAINT unique_scene_name UNIQUE (name);
        END IF;
    END
$$;

CREATE TABLE IF NOT EXISTS scene_participant
(
    uuid        UUID PRIMARY KEY,
    scene_uuid  UUID REFERENCES scene (uuid) ON DELETE CASCADE,
    person_uuid UUID        NOT NULL,
    joined_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    left_at     TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS scene_snapshot
(
    scene_uuid  UUID PRIMARY KEY REFERENCES scene (uuid) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    description TEXT        NOT NULL
);

COMMIT;