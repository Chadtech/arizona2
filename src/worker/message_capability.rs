use crate::capability::message::MessageCapability;
use crate::domain::message::{Message, MessageSender};
use crate::domain::message_uuid::MessageUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::scene_uuid::SceneUuid;
use crate::worker::Worker;
use chrono::{DateTime, Utc};

impl MessageCapability for Worker {
    async fn send_scene_message(
        &self,
        sender: MessageSender,
        scene_uuid: SceneUuid,
        content: String,
    ) -> Result<MessageUuid, String> {
        let message_uuid = MessageUuid::new();

        let sender_uuid = match sender {
            MessageSender::AiPerson(person_uuid) => Some(person_uuid.to_uuid()),
            MessageSender::RealWorldUser => None,
        };

        sqlx::query!(
            r#"
                INSERT INTO message (uuid, sender_person_uuid, scene_uuid, content)
                VALUES ($1::UUID, $2::UUID, $3::UUID, $4::TEXT)
            "#,
            message_uuid.to_uuid(),
            sender_uuid,
            scene_uuid.to_uuid(),
            content
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting scene message: {}", err))?;

        Ok(message_uuid)
    }

    async fn add_scene_message_recipients(
        &self,
        message_uuid: &MessageUuid,
        recipients: Vec<PersonUuid>,
    ) -> Result<(), String> {
        for person_uuid in recipients {
            sqlx::query!(
                r#"
                    INSERT INTO scene_message_recipient (message_uuid, person_uuid)
                    VALUES ($1::UUID, $2::UUID)
                    ON CONFLICT DO NOTHING
                "#,
                message_uuid.to_uuid(),
                person_uuid.to_uuid()
            )
            .execute(&self.sqlx)
            .await
            .map_err(|err| format!("Error inserting scene message recipient: {}", err))?;
        }

        Ok(())
    }

    async fn get_messages_in_scene_page(
        &self,
        scene_uuid: &SceneUuid,
        limit: i64,
        before_sent_at: Option<DateTime<Utc>>,
    ) -> Result<Vec<Message>, String> {
        let rows = sqlx::query!(
            r#"
                SELECT uuid, sender_person_uuid,  scene_uuid, content, sent_at
                FROM message
                WHERE scene_uuid = $1::UUID
                  AND ($2::timestamptz IS NULL OR sent_at < $2::timestamptz)
                ORDER BY sent_at DESC
                LIMIT $3
            "#,
            scene_uuid.to_uuid(),
            before_sent_at,
            limit
        )
        .fetch_all(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching paged messages in scene: {}", err))?;

        Ok(rows
            .into_iter()
            .map(|row| Message {
                uuid: MessageUuid::from_uuid(row.uuid),
                sender: match row.sender_person_uuid {
                    Some(uuid) => MessageSender::AiPerson(PersonUuid::from_uuid(uuid)),
                    None => MessageSender::RealWorldUser,
                },
                scene_uuid: SceneUuid::from_uuid(row.scene_uuid),
                content: row.content,
                sent_at: row.sent_at,
            })
            .collect())
    }

    async fn get_message_by_uuid(
        &self,
        message_uuid: &MessageUuid,
    ) -> Result<Option<Message>, String> {
        let row = sqlx::query!(
            r#"
                SELECT uuid, sender_person_uuid, scene_uuid, content, sent_at
                FROM message
                WHERE uuid = $1::UUID
            "#,
            message_uuid.to_uuid()
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching message by uuid: {}", err))?;

        match row {
            Some(row) => {
                let scene_uuid = SceneUuid::from_uuid(row.scene_uuid);

                Ok(Some(Message {
                    uuid: MessageUuid::from_uuid(row.uuid),
                    sender: match row.sender_person_uuid {
                        Some(uuid) => MessageSender::AiPerson(PersonUuid::from_uuid(uuid)),
                        None => MessageSender::RealWorldUser,
                    },
                    scene_uuid,
                    content: row.content,
                    sent_at: row.sent_at,
                }))
            }
            None => Ok(None),
        }
    }

    async fn get_unhandled_scene_messages_for_person(
        &self,
        person_uuid: &PersonUuid,
        scene_uuid: &SceneUuid,
    ) -> Result<Vec<Message>, String> {
        let rows = sqlx::query!(
            r#"
                SELECT m.uuid, m.sender_person_uuid, m.scene_uuid, m.content, m.sent_at
                FROM message m
                JOIN scene_message_recipient smr ON smr.message_uuid = m.uuid
                WHERE smr.person_uuid = $1::UUID
                  AND smr.handled_at IS NULL
                  AND m.scene_uuid = $2::UUID
                ORDER BY m.sent_at ASC
            "#,
            person_uuid.to_uuid(),
            scene_uuid.to_uuid()
        )
        .fetch_all(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching unhandled scene messages: {}", err))?;

        let messages = rows
            .into_iter()
            .map(|row| Message {
                uuid: MessageUuid::from_uuid(row.uuid),
                sender: match row.sender_person_uuid {
                    Some(uuid) => MessageSender::AiPerson(PersonUuid::from_uuid(uuid)),
                    None => MessageSender::RealWorldUser,
                },
                scene_uuid: SceneUuid::from_uuid(row.scene_uuid),
                content: row.content,
                sent_at: row.sent_at,
            })
            .collect();

        Ok(messages)
    }

    async fn mark_scene_messages_handled_for_person(
        &self,
        person_uuid: &PersonUuid,
        message_uuids: Vec<MessageUuid>,
    ) -> Result<(), String> {
        if message_uuids.is_empty() {
            return Ok(());
        }

        let ids: Vec<uuid::Uuid> = message_uuids
            .into_iter()
            .map(|uuid| uuid.to_uuid())
            .collect();

        sqlx::query!(
            r#"
                UPDATE scene_message_recipient
                SET handled_at = NOW()
                WHERE person_uuid = $1::UUID
                  AND message_uuid = ANY($2::UUID[])
            "#,
            person_uuid.to_uuid(),
            &ids[..]
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error marking scene messages handled: {}", err))?;

        Ok(())
    }
}
