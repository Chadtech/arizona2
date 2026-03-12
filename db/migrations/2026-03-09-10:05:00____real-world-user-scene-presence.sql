CREATE TABLE IF NOT EXISTS real_world_user_scene_presence (
    scene_uuid UUID PRIMARY KEY REFERENCES scene(uuid) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
