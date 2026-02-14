use crate::admin_ui::style as s;
use crate::capability::message::MessageCapability;
use crate::capability::person::PersonCapability;
use crate::capability::scene::SceneCapability;
use crate::domain::message::MessageSender;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::scene_uuid::SceneUuid;
use crate::worker::Worker;
use chrono::{DateTime, Utc};
use iced::clipboard;
use iced::widget::scrollable;
use iced::{widget as w, Element, Length, Task};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Model {
    items: Vec<TimelineItem>,
    scrollable_id: scrollable::Id,
}

#[derive(Debug, Clone)]
pub enum TimelineItem {
    Message {
        sender_label: String,
        content: String,
        timestamp: DateTime<Utc>,
    },
    PersonJoined {
        person_label: String,
        timestamp: DateTime<Utc>,
    },
    PersonLeft {
        person_label: String,
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
    Copy(String),
}

impl Model {
    pub async fn load(worker: &Worker, scene_uuid: SceneUuid) -> Result<Model, String> {
        let mut timeline_items = vec![];
        let mut name_cache: HashMap<String, String> = HashMap::new();
        let mut seen_messages: HashSet<String> = HashSet::new();
        let messages = worker.get_messages_in_scene(&scene_uuid).await?;

        for message in messages {
            let sender_key = match &message.sender {
                MessageSender::AiPerson(uuid) => uuid.to_uuid().to_string(),
                MessageSender::RealWorldUser => "real_world_user".to_string(),
            };
            let message_key = format!(
                "{}|{}|{}",
                sender_key,
                message.content,
                message.sent_at.timestamp()
            );

            if !seen_messages.insert(message_key) {
                continue;
            }

            let sender_label = match &message.sender {
                MessageSender::AiPerson(uuid) => {
                    person_label(worker, &mut name_cache, uuid).await?
                }
                MessageSender::RealWorldUser => "You".to_string(),
            };
            timeline_items.push(TimelineItem::Message {
                sender_label,
                content: message.content,
                timestamp: message.sent_at,
            });
        }

        let participation = worker.get_scene_participation_history(&scene_uuid).await?;

        for event in participation {
            if let Some(left_at) = event.left_at {
                let person_label =
                    person_label(worker, &mut name_cache, &event.person_uuid).await?;
                let left_item = TimelineItem::PersonLeft {
                    person_label,
                    timestamp: left_at,
                };
                timeline_items.push(left_item);
            }

            let person_label = person_label(worker, &mut name_cache, &event.person_uuid).await?;
            let joined_item = TimelineItem::PersonJoined {
                person_label,
                timestamp: event.joined_at,
            };

            timeline_items.push(joined_item);
        }

        timeline_items.sort_by_key(|item| item.posix_timestamp());

        Ok(Model {
            items: timeline_items,
            scrollable_id: scrollable::Id::unique(),
        })
    }

    pub fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Copy(contents) => clipboard::write(contents),
        }
    }

    pub fn scroll_to_bottom(&self) -> Task<Msg> {
        scrollable::snap_to(self.scrollable_id.clone(), scrollable::RelativeOffset::END)
    }

    pub fn view(&self) -> Element<'_, Msg> {
        if self.items.is_empty() {
            w::text("No messages or events in this scene").into()
        } else {
            let timeline = self
                .items
                .iter()
                .fold(w::column![], |col, item| {
                    col.push(self.view_timeline_item(item))
                })
                .spacing(s::S2);

            w::scrollable(timeline)
                .id(self.scrollable_id.clone())
                .width(Length::Fill)
                .height(Length::Fixed(s::LIST_HEIGHT))
                .into()
        }
    }

    fn view_timeline_item<'a>(&'a self, item: &'a TimelineItem) -> Element<'a, Msg> {
        match item {
            TimelineItem::Message {
                sender_label,
                content,
                timestamp,
            } => {
                let display_content = normalize_message_content(content);
                let time_str = timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
                let is_you = sender_label == "You";
                let name_color = if is_you { s::GREEN_SOFT } else { s::GOLD_SOFT };
                let header_text = format!("[{}] {}", time_str, sender_label);
                let copy_text = format!("{}\n{}", header_text, display_content);

                w::column![
                    w::row![
                        w::text(sender_label).size(s::S4).color(name_color),
                    ]
                    .spacing(s::S1),
                    w::row![
                        w::text(format!("[{}]", time_str))
                            .size(s::S3)
                            .color(s::GRAY_MID),
                        w::button(w::text("Copy").size(s::S3))
                            .style(w::button::text)
                            .padding(0)
                            .on_press(Msg::Copy(copy_text)),
                    ]
                    .spacing(s::S1),
                    w::text(display_content),
                ]
                .spacing(s::S1)
                .padding(s::S1)
                .into()
            }
            TimelineItem::PersonJoined {
                person_label,
                timestamp,
            } => {
                let time_str = timestamp.format("%H:%M:%S").to_string();
                let message = format!("[{}] → {} joined the scene", time_str, person_label);
                let copy_text = message.clone();
                w::row![
                    w::text(message).size(s::S3),
                    w::button(w::text("Copy").size(s::S3))
                        .style(w::button::text)
                        .padding(0)
                        .on_press(Msg::Copy(copy_text)),
                ]
                .spacing(s::S1)
                .padding(s::S1)
                .into()
            }
            TimelineItem::PersonLeft {
                person_label,
                timestamp,
            } => {
                let time_str = timestamp.format("%H:%M:%S").to_string();
                let message = format!("[{}] ← {} left the scene", time_str, person_label);
                let copy_text = message.clone();
                w::row![
                    w::text(message).size(s::S3),
                    w::button(w::text("Copy").size(s::S3))
                        .style(w::button::text)
                        .padding(0)
                        .on_press(Msg::Copy(copy_text)),
                ]
                .spacing(s::S1)
                .padding(s::S1)
                .into()
            }
        }
    }
}

fn normalize_message_content(content: &str) -> &str {
    let bytes = content.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if first == b'"' && last == b'"' {
            return &content[1..bytes.len() - 1];
        }
    }

    content
}

async fn person_label(
    worker: &Worker,
    cache: &mut HashMap<String, String>,
    person_uuid: &PersonUuid,
) -> Result<String, String> {
    let key = person_uuid.to_string();
    if let Some(label) = cache.get(&key) {
        return Ok(label.clone());
    }

    let name = worker.get_persons_name(person_uuid.clone()).await?;
    let label = name.as_str().to_string();
    cache.insert(key, label.clone());
    Ok(label)
}
