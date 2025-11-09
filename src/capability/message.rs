use crate::domain::message::{Message, MessageRecipient, MessageSender};
use crate::domain::message_uuid::MessageUuid;
use async_trait::async_trait;

pub struct NewMessage {
    pub sender: MessageSender,
    pub recipient: MessageRecipient,
    pub content: String,
}

#[async_trait]
pub trait MessageCapability {
    async fn send_message(
        &self,
        new_message: NewMessage,
    ) -> Result<MessageUuid, String>;
    async fn get_messages_in_scene(
        &self,
        scene_uuid: &crate::domain::scene_uuid::SceneUuid,
    ) -> Result<Vec<Message>, String>;
}
