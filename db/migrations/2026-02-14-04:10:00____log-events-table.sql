-- log-events-table

BEGIN;

CREATE TABLE IF NOT EXISTS log_event
(
    uuid       UUID PRIMARY KEY,
    event_name TEXT        NOT NULL,
    data       JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_log_event_name ON log_event (event_name);
CREATE INDEX IF NOT EXISTS idx_log_event_created_at ON log_event (created_at);

COMMIT;
