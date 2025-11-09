use super::message_uuid::MessageUuid;
use super::person_uuid::PersonUuid;
use super::scene_uuid::SceneUuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Message {
    pub uuid: MessageUuid,
    pub sender_person_uuid: PersonUuid,
    pub recipient: MessageRecipient,
    pub content: String,
    pub sent_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub enum MessageRecipient {
    Person(PersonUuid),
    Scene(SceneUuid),
    RealWorldPerson, // Message to the actual user (Chad)
}
