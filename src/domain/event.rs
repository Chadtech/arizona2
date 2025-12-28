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
                scene_uuid,
                comment,
            } => {
                format!(
                    "At {}, in scene {}, someone said: {}",
                    self.timestamp,
                    scene_uuid.to_uuid(),
                    comment
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
                scene_uuid,
            } => {
                format!(
                    "At {}, person {} joined scene {}",
                    self.timestamp,
                    person_uuid.to_uuid(),
                    scene_uuid.to_uuid()
                )
            }
            EventType::PersonLeftScene {
                person_uuid,
                scene_uuid,
            } => {
                format!(
                    "At {}, person {} left scene {}",
                    self.timestamp,
                    person_uuid.to_uuid(),
                    scene_uuid.to_uuid()
                )
            }
        }
    }
}

pub enum EventType {
    PersonSaidInScene {
        scene_uuid: SceneUuid,
        comment: String,
    },
    PersonDirectMessaged {
        sender: MessageSender,
        comment: String,
    },
    PersonJoinedScene {
        person_uuid: PersonUuid,
        scene_uuid: SceneUuid,
    },
    PersonLeftScene {
        person_uuid: PersonUuid,
        scene_uuid: SceneUuid,
    },
}
