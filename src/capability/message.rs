use crate::domain::message::{Message, MessageRecipient, MessageSender};
use crate::domain::message_uuid::MessageUuid;
use crate::domain::scene_uuid::SceneUuid;

pub struct NewMessage {
    pub sender: MessageSender,
    pub recipient: MessageRecipient,
    pub content: String,
    pub scene_uuid: Option<SceneUuid>,
}

pub trait MessageCapability {
    async fn send_message(&self, new_message: NewMessage) -> Result<MessageUuid, String>;
    async fn get_messages_in_scene(&self, scene_uuid: &SceneUuid) -> Result<Vec<Message>, String>;
    async fn get_message_by_uuid(
        &self,
        message_uuid: &MessageUuid,
    ) -> Result<Option<Message>, String>;
}
