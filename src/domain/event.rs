use super::message::MessageSender;
use crate::domain::message_uuid::MessageUuid;
use crate::domain::person_uuid::PersonUuid;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct Event {
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
}

impl Event {
    pub fn new(timestamp: DateTime<Utc>, event_type: EventType) -> Self {
        Self {
            timestamp,
            event_type,
        }
    }

    pub fn to_text(&self) -> String {
        match &self.event_type {
            EventType::PersonSaidInScene {
                scene_name,
                speaker_name,
                comment,
                message_uuid: _,
            } => {
                format!(
                    "At {}, in scene {}, {} said: \"{}\"",
                    self.timestamp, scene_name, speaker_name, comment
                )
            }
            EventType::PersonDirectMessaged {
                sender,
                comment,
                message_uuid: _,
            } => {
                format!(
                    "At {}, {} sent a direct message: {}",
                    self.timestamp,
                    sender.to_string(),
                    comment
                )
            }
            EventType::PersonJoinedScene {
                person_uuid: _,
                person_name,
                scene_name,
            } => {
                format!(
                    "At {}, {} joined scene {}",
                    self.timestamp,
                    person_name,
                    scene_name
                )
            }
            EventType::PersonLeftScene {
                person_uuid: _,
                person_name,
                scene_name,
            } => {
                format!(
                    "At {}, {} left scene {}",
                    self.timestamp,
                    person_name,
                    scene_name
                )
            }
        }
    }

    pub fn many_to_prompt_list(events: Vec<Event>) -> String {
        if events.is_empty() {
            "None.".to_string()
        } else {
            events
                .iter()
                .map(|event| event.to_text())
                .rev()
                .take(8)
                .collect::<Vec<String>>()
                .into_iter()
                .rev()
                .collect::<Vec<String>>()
                .join("\n")
        }
    }
}

#[derive(Clone, Debug)]
pub enum EventType {
    PersonSaidInScene {
        scene_name: String,
        speaker_name: String,
        comment: String,
        message_uuid: MessageUuid,
    },
    PersonDirectMessaged {
        sender: MessageSender,
        comment: String,
        message_uuid: MessageUuid,
    },
    PersonJoinedScene {
        person_uuid: PersonUuid,
        person_name: String,
        scene_name: String,
    },
    PersonLeftScene {
        person_uuid: PersonUuid,
        person_name: String,
        scene_name: String,
    },
}
