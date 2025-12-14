use crate::capability::message::{MessageCapability, NewMessage};
use crate::domain::message::{Message, MessageRecipient, MessageSender};
use crate::domain::message_uuid::MessageUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::scene_uuid::SceneUuid;
use crate::worker::Worker;

impl MessageCapability for Worker {
    async fn send_message(&self, new_message: NewMessage) -> Result<MessageUuid, String> {
        let message_uuid = MessageUuid::new();

        let scene_uuid = new_message.scene_uuid.map(|s| s.to_uuid());

        let receiver_uuid = match new_message.recipient {
            MessageRecipient::Person(receiver) => Some(receiver.to_uuid()),
            MessageRecipient::RealWorldUser => None,
        };

        let sender_uuid = match new_message.sender {
            MessageSender::AiPerson(person_uuid) => Some(person_uuid.to_uuid()),
            MessageSender::RealWorldUser => None,
        };

        sqlx::query!(
            r#"
                INSERT INTO message (uuid, sender_person_uuid, receiver_person_uuid, scene_uuid, content)
                VALUES ($1::UUID, $2::UUID, $3::UUID, $4::UUID, $5::TEXT)
            "#,
            message_uuid.to_uuid(),
            sender_uuid,
            receiver_uuid,
            scene_uuid,
            new_message.content,
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting message: {}", err))?;

        Ok(message_uuid)
    }

    async fn get_messages_in_scene(&self, scene_uuid: &SceneUuid) -> Result<Vec<Message>, String> {
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
                recipient: match row.receiver_person_uuid {
                    Some(uuid) => MessageRecipient::Person(PersonUuid::from_uuid(uuid)),
                    None => MessageRecipient::RealWorldUser,
                },
                scene_uuid: match row.scene_uuid {
                    Some(uuid) => Some(SceneUuid::from_uuid(uuid)),
                    None => None,
                },
                content: row.content,
                sent_at: row.sent_at,
                read_at: row.read_at,
            })
            .collect();

        Ok(messages)
    }

    async fn get_message_by_uuid(
        &self,
        message_uuid: &MessageUuid,
    ) -> Result<Option<Message>, String> {
        let row = sqlx::query!(
            r#"
                SELECT uuid, sender_person_uuid, receiver_person_uuid, scene_uuid, content, sent_at, read_at
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
                let recipient = match row.receiver_person_uuid {
                    Some(uuid) => MessageRecipient::Person(PersonUuid::from_uuid(uuid)),
                    None => MessageRecipient::RealWorldUser,
                };

                Ok(Some(Message {
                    uuid: MessageUuid::from_uuid(row.uuid),
                    sender: match row.sender_person_uuid {
                        Some(uuid) => MessageSender::AiPerson(PersonUuid::from_uuid(uuid)),
                        None => MessageSender::RealWorldUser,
                    },
                    scene_uuid: match row.scene_uuid {
                        Some(uuid) => Some(SceneUuid::from_uuid(uuid)),
                        None => None,
                    },
                    recipient,
                    content: row.content,
                    sent_at: row.sent_at,
                    read_at: row.read_at,
                }))
            }
            None => Ok(None),
        }
    }
}
