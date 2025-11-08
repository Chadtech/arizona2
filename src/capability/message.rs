use crate::domain::message::{Message, MessageRecipient};
use crate::domain::message_uuid::MessageUuid;
use crate::domain::person_uuid::PersonUuid;
use async_trait::async_trait;

pub struct NewMessage {
    pub recipient: MessageRecipient,
    pub content: String,
}

#[async_trait]
pub trait MessageCapability {
    async fn send_message(
        &self,
        sender_person_uuid: &PersonUuid,
        new_message: NewMessage,
    ) -> Result<MessageUuid, String>;
    async fn get_messages_in_scene(
        &self,
        scene_uuid: &crate::domain::scene_uuid::SceneUuid,
    ) -> Result<Vec<Message>, String>;
}
