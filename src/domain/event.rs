use super::{message::MessageSender, scene_uuid::SceneUuid};
use crate::domain::person_uuid::PersonUuid;
use chrono::{DateTime, Utc};

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
        // TODO, add names to all these uuids so that the events can be human readable
        match &self.event_type {
            EventType::PersonSaidInScene {
                scene_name,
                speaker_name,
                comment,
            } => {
                format!(
                    "At {}, in scene {}, {} said: {}",
                    self.timestamp, scene_name, speaker_name, comment
                )
            }
            EventType::PersonDirectMessaged { sender, comment } => {
                format!(
                    "At {}, {} sent a direct message: {}",
                    self.timestamp,
                    sender.to_string(),
                    comment
                )
            }
            EventType::PersonJoinedScene {
                person_uuid,
                scene_uuid: _,
                scene_name,
            } => {
                format!(
                    "At {}, person {} joined scene {}",
                    self.timestamp,
                    person_uuid.to_uuid(),
                    scene_name
                )
            }
            EventType::PersonLeftScene {
                person_uuid,
                scene_uuid: _,
                scene_name,
            } => {
                format!(
                    "At {}, person {} left scene {}",
                    self.timestamp,
                    person_uuid.to_uuid(),
                    scene_name
                )
            }
        }
    }
}

pub enum EventType {
    PersonSaidInScene {
        scene_name: String,
        speaker_name: String,
        comment: String,
    },
    PersonDirectMessaged {
        sender: MessageSender,
        comment: String,
    },
    PersonJoinedScene {
        person_uuid: PersonUuid,
        scene_uuid: SceneUuid,
        scene_name: String,
    },
    PersonLeftScene {
        person_uuid: PersonUuid,
        scene_uuid: SceneUuid,
        scene_name: String,
    },
}
