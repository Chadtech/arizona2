use crate::domain::message::{Message, MessageRecipient, MessageSender};
use crate::domain::message_uuid::MessageUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::scene_uuid::SceneUuid;

pub struct NewMessage {
    pub sender: MessageSender,
    pub recipient: MessageRecipient,
    pub content: String,
    pub scene_uuid: Option<SceneUuid>,
}

pub trait MessageCapability {
    async fn send_message(&self, new_message: NewMessage) -> Result<MessageUuid, String>;
    async fn send_scene_message(
        &self,
        sender: MessageSender,
        scene_uuid: SceneUuid,
        content: String,
    ) -> Result<MessageUuid, String>;
    async fn add_scene_message_recipients(
        &self,
        message_uuid: &MessageUuid,
        recipients: Vec<PersonUuid>,
    ) -> Result<(), String>;
    async fn get_messages_in_scene(&self, scene_uuid: &SceneUuid) -> Result<Vec<Message>, String>;
    async fn get_message_by_uuid(
        &self,
        message_uuid: &MessageUuid,
    ) -> Result<Option<Message>, String>;
    async fn mark_message_read(&self, message_uuid: &MessageUuid) -> Result<(), String>;
    async fn get_unhandled_scene_messages_for_person(
        &self,
        person_uuid: &PersonUuid,
        scene_uuid: &SceneUuid,
    ) -> Result<Vec<Message>, String>;
    async fn mark_scene_messages_handled_for_person(
        &self,
        person_uuid: &PersonUuid,
        message_uuids: Vec<MessageUuid>,
    ) -> Result<(), String>;
}
