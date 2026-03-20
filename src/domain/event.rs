use crate::domain::message_uuid::MessageUuid;
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
            EventType::Said {
                scene_name,
                speaker_name,
                comment,
                message_uuid: _,
            } => format!(
                "In scene {}, {} said: \"{}\"",
                scene_name, speaker_name, comment
            ),
            EventType::Entered {
                person_name,
                scene_name,
            } => format!("{} entered scene {}", person_name, scene_name),
            EventType::Left {
                person_name,
                scene_name,
            } => format!("{} left scene {}", person_name, scene_name),
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
    Said {
        scene_name: String,
        speaker_name: String,
        comment: String,
        message_uuid: MessageUuid,
    },
    Entered {
        person_name: String,
        scene_name: String,
    },
    Left {
        person_name: String,
        scene_name: String,
    },
}
