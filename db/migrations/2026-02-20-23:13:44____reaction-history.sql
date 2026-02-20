-- reaction-history

CREATE TABLE IF NOT EXISTS reaction_history (
    uuid UUID PRIMARY KEY,
    person_uuid UUID NOT NULL,
    action_kind TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS reaction_history_person_time_idx
    ON reaction_history (person_uuid, created_at DESC);
