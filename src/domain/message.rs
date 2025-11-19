use super::message_uuid::MessageUuid;
use super::person_uuid::PersonUuid;
use super::scene_uuid::SceneUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Message {
    pub uuid: MessageUuid,
    pub sender: MessageSender,
    pub recipient: MessageRecipient,
    pub scene_uuid: Option<SceneUuid>,
    pub content: String,
    pub sent_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageSender {
    AiPerson(PersonUuid),
    RealWorldUser, // Represents Chad or other real users
}

#[derive(Debug, Clone)]
pub enum MessageRecipient {
    Person(PersonUuid),
    RealWorldPerson, // Message to the actual user (Chad)
}
