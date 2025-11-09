use crate::capability::message::{MessageCapability, NewMessage};
use crate::domain::message::{Message, MessageRecipient, MessageSender};
use crate::domain::message_uuid::MessageUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::worker::Worker;
use async_trait::async_trait;

#[async_trait]
impl MessageCapability for Worker {
    async fn send_message(&self, new_message: NewMessage) -> Result<MessageUuid, String> {
        let message_uuid = MessageUuid::new();

        let (message_type, receiver_uuid, scene_uuid) = match new_message.recipient {
            MessageRecipient::Person(receiver) => ("direct", Some(receiver.to_uuid()), None),
            MessageRecipient::Scene(scene) => ("scene_broadcast", None, Some(scene.to_uuid())),
            MessageRecipient::RealWorldPerson => ("to_user", None, None),
        };

        let sender_uuid = match new_message.sender {
            MessageSender::AiPerson(person_uuid) => Some(person_uuid.to_uuid()),
            MessageSender::RealWorldUser => None,
        };

        sqlx::query!(
            r#"
                INSERT INTO message (uuid, sender_person_uuid, receiver_person_uuid, scene_uuid, content, message_type)
                VALUES ($1::UUID, $2::UUID, $3::UUID, $4::UUID, $5::TEXT, $6::TEXT)
            "#,
            message_uuid.to_uuid(),
            sender_uuid,
            receiver_uuid,
            scene_uuid,
            new_message.content,
            message_type
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting message: {}", err))?;

        Ok(message_uuid)
    }

    async fn get_messages_in_scene(
        &self,
        scene_uuid: &crate::domain::scene_uuid::SceneUuid,
    ) -> Result<Vec<Message>, String> {
        let rows = sqlx::query!(
            r#"
                SELECT uuid, sender_person_uuid, receiver_person_uuid, scene_uuid, content, sent_at, read_at
                FROM message
                WHERE scene_uuid = $1::UUID
                ORDER BY sent_at ASC
            "#,
            scene_uuid.to_uuid()
        )
        .fetch_all(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching messages in scene: {}", err))?;

        let messages = rows
            .into_iter()
            .map(|row| Message {
                uuid: MessageUuid::from_uuid(row.uuid),
                sender: match row.sender_person_uuid {
                    Some(uuid) => MessageSender::AiPerson(PersonUuid::from_uuid(uuid)),
                    None => MessageSender::RealWorldUser,
                },
                recipient: MessageRecipient::Scene(
                    crate::domain::scene_uuid::SceneUuid::from_uuid(row.scene_uuid.unwrap()),
                ),
                content: row.content,
                sent_at: row.sent_at,
                read_at: row.read_at,
            })
            .collect();

        Ok(messages)
    }
}
