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

pub const MESSAGE_PAGE_SIZE: usize = 16;
const LOAD_MORE_THRESHOLD: f32 = 0.05;

#[derive(Debug, Clone)]
pub struct Model {
    items: Vec<TimelineItem>,
    scrollable_id: scrollable::Id,
    oldest_message_at: Option<DateTime<Utc>>,
    loading_older: bool,
    has_more_messages: bool,
    seen_messages: HashSet<String>,
    can_load_older: bool,
    pending_content_height: Option<f32>,
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
    Scrolled(scrollable::Viewport),
}

#[derive(Debug, Clone)]
pub struct LoadOlderResult {
    pub items: Vec<TimelineItem>,
    pub keys: Vec<String>,
    pub oldest_message_at: Option<DateTime<Utc>>,
    pub has_more: bool,
}

pub enum ScrollDecision {
    None,
    LoadOlder,
    AdjustScroll(f32),
}

impl Model {
    pub async fn load(worker: &Worker, scene_uuid: SceneUuid) -> Result<Model, String> {
        let mut timeline_items = vec![];
        let load_result = load_message_page(
            worker,
            scene_uuid.clone(),
            MESSAGE_PAGE_SIZE,
            None,
            HashSet::new(),
        )
        .await?;

        timeline_items.extend(load_result.items.iter().cloned());

        let participation = worker.get_scene_participation_history(&scene_uuid).await?;

        let mut name_cache: HashMap<String, String> = HashMap::new();
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
            oldest_message_at: load_result.oldest_message_at,
            loading_older: false,
            has_more_messages: load_result.has_more,
            seen_messages: load_result.keys.into_iter().collect(),
            can_load_older: true,
            pending_content_height: None,
        })
    }

    pub fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Copy(contents) => clipboard::write(contents),
            Msg::Scrolled(_) => Task::none(),
        }
    }

    pub fn scroll_to_bottom(&self) -> Task<Msg> {
        scrollable::snap_to(self.scrollable_id.clone(), scrollable::RelativeOffset::END)
    }

    pub fn oldest_message_at(&self) -> Option<DateTime<Utc>> {
        self.oldest_message_at
    }

    pub fn handle_scroll(&mut self, viewport: scrollable::Viewport) -> ScrollDecision {
        if let Some(previous_height) = self.pending_content_height {
            self.pending_content_height = None;
            let delta = viewport.content_bounds().height - previous_height;
            if delta > 0.0 {
                return ScrollDecision::AdjustScroll(delta);
            }
        }

        let relative_offset = viewport.relative_offset();
        if relative_offset.y > LOAD_MORE_THRESHOLD {
            self.can_load_older = true;
            return ScrollDecision::None;
        }

        if self.loading_older || !self.has_more_messages || !self.can_load_older {
            return ScrollDecision::None;
        }

        self.can_load_older = false;
        self.pending_content_height = Some(viewport.content_bounds().height);
        ScrollDecision::LoadOlder
    }

    pub fn mark_loading_older(&mut self) {
        self.loading_older = true;
    }

    pub fn apply_older_messages(&mut self, result: LoadOlderResult) {
        self.loading_older = false;
        self.has_more_messages = result.has_more;
        if result.items.is_empty() {
            self.has_more_messages = false;
        }

        for key in result.keys {
            self.seen_messages.insert(key);
        }

        self.items.extend(result.items);
        self.items.sort_by_key(|item| item.posix_timestamp());

        if let Some(oldest) = result.oldest_message_at {
            match self.oldest_message_at {
                Some(current) => {
                    if oldest < current {
                        self.oldest_message_at = Some(oldest);
                    }
                }
                None => {
                    self.oldest_message_at = Some(oldest);
                }
            }
        }
    }

    pub fn finish_loading_older_error(&mut self) {
        self.loading_older = false;
    }

    pub fn seen_message_keys(&self) -> HashSet<String> {
        self.seen_messages.clone()
    }

    pub fn scroll_by(&self, delta: f32) -> Task<Msg> {
        scrollable::scroll_by(
            self.scrollable_id.clone(),
            scrollable::AbsoluteOffset { x: 0.0, y: delta },
        )
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
                .on_scroll(Msg::Scrolled)
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

pub async fn load_older_messages(
    worker: &Worker,
    scene_uuid: SceneUuid,
    before: DateTime<Utc>,
    known_keys: HashSet<String>,
) -> Result<LoadOlderResult, String> {
    load_message_page(
        worker,
        scene_uuid,
        MESSAGE_PAGE_SIZE,
        Some(before),
        known_keys,
    )
    .await
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

async fn load_message_page(
    worker: &Worker,
    scene_uuid: SceneUuid,
    limit: usize,
    before: Option<DateTime<Utc>>,
    known_keys: HashSet<String>,
) -> Result<LoadOlderResult, String> {
    let mut name_cache: HashMap<String, String> = HashMap::new();
    let mut items = Vec::new();
    let mut keys = Vec::new();
    let mut oldest_message_at: Option<DateTime<Utc>> = None;

    let messages = worker
        .get_messages_in_scene_page(&scene_uuid, limit as i64, before)
        .await?;

    for message in messages.iter() {
        let message_key = message_key(message);
        if known_keys.contains(&message_key) {
            continue;
        }

        let sender_label = match &message.sender {
            MessageSender::AiPerson(uuid) => person_label(worker, &mut name_cache, uuid).await?,
            MessageSender::RealWorldUser => "You".to_string(),
        };

        items.push(TimelineItem::Message {
            sender_label,
            content: message.content.clone(),
            timestamp: message.sent_at,
        });
        keys.push(message_key);

        oldest_message_at = match oldest_message_at {
            Some(oldest) => {
                if message.sent_at < oldest {
                    Some(message.sent_at)
                } else {
                    Some(oldest)
                }
            }
            None => Some(message.sent_at),
        };
    }

    Ok(LoadOlderResult {
        items,
        keys,
        oldest_message_at,
        has_more: messages.len() == limit,
    })
}

fn message_key(message: &crate::domain::message::Message) -> String {
    let sender_key = match &message.sender {
        MessageSender::AiPerson(uuid) => uuid.to_uuid().to_string(),
        MessageSender::RealWorldUser => "real_world_user".to_string(),
    };

    format!(
        "{}|{}|{}",
        sender_key,
        message.content,
        message.sent_at.timestamp()
    )
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
