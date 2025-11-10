-- message-table

BEGIN;

-- Ensure vector extension is available (for optional semantic search)
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS message
(
    uuid                 UUID PRIMARY KEY,
    sender_person_uuid   UUID,
    receiver_person_uuid UUID,
    content              TEXT        NOT NULL,
    scene_uuid           UUID,
    message_type         TEXT        NOT NULL,
    sent_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    read_at              TIMESTAMPTZ,
    -- Optional: vector for semantic search of messages (like memory table)
    embedding            vector(1536),

    -- Ensure message_type is one of the valid values
    CONSTRAINT valid_message_type CHECK (message_type IN ('direct', 'scene_broadcast', 'to_user')),

    -- Ensure 'direct' messages have a receiver and no scene
    CONSTRAINT check_direct_message CHECK (
        message_type != 'direct' OR
        (receiver_person_uuid IS NOT NULL AND scene_uuid IS NULL)
        ),

    -- Ensure 'scene_broadcast' messages have a scene and no specific receiver
    CONSTRAINT check_scene_broadcast CHECK (
        message_type != 'scene_broadcast' OR
        (scene_uuid IS NOT NULL AND receiver_person_uuid IS NULL)
        ),

    -- Ensure 'to_user' messages have no receiver and no scene
    CONSTRAINT check_to_user CHECK (
        message_type != 'to_user' OR
        (receiver_person_uuid IS NULL AND scene_uuid IS NULL)
        )
);

-- Foreign key: sender must be a person
DO
$$
    BEGIN
        IF NOT EXISTS (SELECT 1
                       FROM pg_constraint
                       WHERE conname = 'message_fk_sender_person') THEN
            ALTER TABLE message
                ADD CONSTRAINT message_fk_sender_person
                    FOREIGN KEY (sender_person_uuid)
                        REFERENCES person (uuid)
                        ON DELETE CASCADE;
        END IF;
    END
$$;

-- Foreign key: receiver can be a person (NULL for scene_broadcast or to_user messages)
DO
$$
    BEGIN
        IF NOT EXISTS (SELECT 1
                       FROM pg_constraint
                       WHERE conname = 'message_fk_receiver_person') THEN
            ALTER TABLE message
                ADD CONSTRAINT message_fk_receiver_person
                    FOREIGN KEY (receiver_person_uuid)
                        REFERENCES person (uuid)
                        ON DELETE CASCADE;
        END IF;
    END
$$;

-- Foreign key: message can optionally be associated with a scene
DO
$$
    BEGIN
        IF NOT EXISTS (SELECT 1
                       FROM pg_constraint
                       WHERE conname = 'message_fk_scene') THEN
            ALTER TABLE message
                ADD CONSTRAINT message_fk_scene
                    FOREIGN KEY (scene_uuid)
                        REFERENCES scene (uuid)
                        ON DELETE CASCADE;
        END IF;
    END
$$;

-- Basic indexes for foreign keys (important for JOINs and CASCADE DELETE performance)
CREATE INDEX IF NOT EXISTS idx_message_sender ON message (sender_person_uuid);
CREATE INDEX IF NOT EXISTS idx_message_receiver ON message (receiver_person_uuid);
CREATE INDEX IF NOT EXISTS idx_message_scene ON message (scene_uuid);

COMMIT;
