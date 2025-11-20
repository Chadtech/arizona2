use super::person_uuid::PersonUuid;
use super::scene_uuid::SceneUuid;
use super::{actor_uuid::ActorUuid, message_uuid::MessageUuid};
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

impl MessageSender {
    pub fn to_string(&self) -> String {
        match self {
            MessageSender::AiPerson(person_uuid) => format!("AI Person {}", person_uuid.to_uuid()),
            MessageSender::RealWorldUser => "Real World User".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MessageRecipient {
    Person(PersonUuid),
    RealWorldUser, // Message to the actual user (Chad)
}

impl From<&ActorUuid> for MessageRecipient {
    fn from(actor_uuid: &ActorUuid) -> Self {
        match actor_uuid {
            ActorUuid::AiPerson(person_uuid) => MessageRecipient::Person(person_uuid.clone()),
            ActorUuid::RealWorldUser => MessageRecipient::RealWorldUser,
        }
    }
}
