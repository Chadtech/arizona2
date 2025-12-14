use crate::admin_ui::style as s;
use crate::capability::message::MessageCapability;
use crate::capability::scene::SceneCapability;
use crate::domain::message::MessageSender;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::scene_uuid::SceneUuid;
use crate::worker::Worker;
use chrono::{DateTime, Utc};
use iced::{widget as w, Element, Length, Task};

#[derive(Debug, Clone)]
pub struct Model {
    items: Vec<TimelineItem>,
}

#[derive(Debug, Clone)]
pub enum TimelineItem {
    Message {
        sender: MessageSender,
        content: String,
        timestamp: DateTime<Utc>,
    },
    PersonJoined {
        person_uuid: PersonUuid,
        timestamp: DateTime<Utc>,
    },
    PersonLeft {
        person_uuid: PersonUuid,
        timestamp: DateTime<Utc>,
    },
}

impl TimelineItem {
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            TimelineItem::Message { timestamp, .. } => *timestamp,
            TimelineItem::PersonJoined { timestamp, .. } => *timestamp,
            TimelineItem::PersonLeft { timestamp, .. } => *timestamp,
        }
    }

    pub fn posix_timestamp(&self) -> i64 {
        self.timestamp().timestamp()
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    // Placeholder for future interactions
}

impl Model {
    pub async fn load(worker: &Worker, scene_uuid: SceneUuid) -> Result<Model, String> {
        let mut timeline_items = vec![];
        let messages = worker.get_messages_in_scene(&scene_uuid).await?;

        for message in messages {
            timeline_items.push(TimelineItem::Message {
                sender: message.sender,
                content: message.content,
                timestamp: message.sent_at,
            });
        }

        let participation = worker.get_scene_participation_history(&scene_uuid).await?;

        for event in participation {
            if let Some(left_at) = event.left_at {
                let left_item = TimelineItem::PersonLeft {
                    person_uuid: event.person_uuid.clone(),
                    timestamp: left_at,
                };
                timeline_items.push(left_item);
            }

            let joined_item = TimelineItem::PersonJoined {
                person_uuid: event.person_uuid,
                timestamp: event.joined_at,
            };

            timeline_items.push(joined_item);
        }

        timeline_items.sort_by_key(|item| item.posix_timestamp());

        Ok(Model {
            items: timeline_items,
        })
    }
    pub fn new(items: Vec<TimelineItem>) -> Self {
        Self { items }
    }

    pub fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            // No messages yet
        }
    }

    pub fn view(&self) -> Element<'_, Msg> {
        if self.items.is_empty() {
            w::text("No messages or events in this scene").into()
        } else {
            let timeline = self.items.iter().fold(w::column![], |col, item| {
                col.push(self.view_timeline_item(item))
            });

            w::scrollable(timeline).width(Length::Fill).into()
        }
    }

    fn view_timeline_item<'a>(&'a self, item: &'a TimelineItem) -> Element<'a, Msg> {
        match item {
            TimelineItem::Message {
                sender,
                content,
                timestamp,
            } => {
                let time_str = timestamp.format("%H:%M:%S").to_string();

                let sender_name = match sender {
                    MessageSender::AiPerson(uuid) => {
                        format!("AI Person {}", uuid.to_uuid())
                    }
                    MessageSender::RealWorldUser => "You".to_string(),
                };
                w::column![
                    w::text(format!("[{}] {}", time_str, sender_name)).size(12),
                    w::text(content),
                ]
                .spacing(s::S1)
                .into()
            }
            TimelineItem::PersonJoined {
                person_uuid,
                timestamp,
            } => {
                let time_str = timestamp.format("%H:%M:%S").to_string();
                w::text(format!(
                    "[{}] → Person {} joined the scene",
                    time_str,
                    person_uuid.to_uuid()
                ))
                .size(12)
                .into()
            }
            TimelineItem::PersonLeft {
                person_uuid,
                timestamp,
            } => {
                let time_str = timestamp.format("%H:%M:%S").to_string();
                w::text(format!(
                    "[{}] ← Person {} left the scene",
                    time_str,
                    person_uuid.to_uuid()
                ))
                .size(12)
                .into()
            }
        }
    }
}
