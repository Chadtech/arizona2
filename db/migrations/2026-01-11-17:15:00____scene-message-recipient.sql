-- scene-message-recipient

BEGIN;

CREATE TABLE IF NOT EXISTS scene_message_recipient (
    message_uuid UUID NOT NULL,
    person_uuid UUID NOT NULL,
    handled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (message_uuid, person_uuid),
    CONSTRAINT scene_message_recipient_message_fk
        FOREIGN KEY (message_uuid) REFERENCES message (uuid)
            ON DELETE CASCADE,
    CONSTRAINT scene_message_recipient_person_fk
        FOREIGN KEY (person_uuid) REFERENCES person (uuid)
            ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_scene_message_recipient_person
    ON scene_message_recipient (person_uuid);

CREATE INDEX IF NOT EXISTS idx_scene_message_recipient_handled
    ON scene_message_recipient (handled_at);

COMMIT;
