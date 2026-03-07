use crate::domain::message::{Message, MessageSender};
use crate::domain::message_uuid::MessageUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::scene_uuid::SceneUuid;
use chrono::{DateTime, Utc};

pub trait MessageCapability {
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
    async fn get_messages_in_scene_page(
        &self,
        scene_uuid: &SceneUuid,
        limit: i64,
        before_sent_at: Option<DateTime<Utc>>,
    ) -> Result<Vec<Message>, String>;
    async fn get_message_by_uuid(
        &self,
        message_uuid: &MessageUuid,
    ) -> Result<Option<Message>, String>;
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
