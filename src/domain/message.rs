use super::message_uuid::MessageUuid;
use super::person_uuid::PersonUuid;
use super::scene_uuid::SceneUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct Message {
    pub uuid: MessageUuid,
    pub sender: MessageSender,
    pub scene_uuid: SceneUuid,
    pub content: String,
    pub sent_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageSender {
    AiPerson(PersonUuid),
    RealWorldUser, // Represents Chad or other real users
}

impl Display for MessageSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            MessageSender::AiPerson(person_uuid) => format!("AI Person {}", person_uuid.to_uuid()),
            MessageSender::RealWorldUser => "Real World User".to_string(),
        };
        write!(f, "{}", s)
    }
}
