use super::s;
use iced::{widget as w, Element, Length, Task};

pub struct Model {
    items: Vec<TimelineItem>,
}

#[derive(Debug, Clone)]
pub enum TimelineItem {
    Message {
        sender_name: String,
        content: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    PersonJoined {
        person_name: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    PersonLeft {
        person_name: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

#[derive(Debug, Clone)]
pub enum Msg {
    // Placeholder for future interactions
}

impl Model {
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

            w::scrollable(timeline)
                .width(Length::Fill)
                .into()
        }
    }

    fn view_timeline_item<'a>(&'a self, item: &'a TimelineItem) -> Element<'a, Msg> {
        match item {
            TimelineItem::Message {
                sender_name,
                content,
                timestamp,
            } => {
                let time_str = timestamp.format("%H:%M:%S").to_string();
                w::column![
                    w::text(format!("[{}] {}", time_str, sender_name)).size(12),
                    w::text(content),
                ]
                .spacing(s::S1)
                .into()
            }
            TimelineItem::PersonJoined {
                person_name,
                timestamp,
            } => {
                let time_str = timestamp.format("%H:%M:%S").to_string();
                w::text(format!("[{}] → {} joined the scene", time_str, person_name))
                    .size(12)
                    .into()
            }
            TimelineItem::PersonLeft {
                person_name,
                timestamp,
            } => {
                let time_str = timestamp.format("%H:%M:%S").to_string();
                w::text(format!("[{}] ← {} left the scene", time_str, person_name))
                    .size(12)
                    .into()
            }
        }
    }
}
