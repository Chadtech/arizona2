use super::s;
use crate::worker::Worker;
use iced::{widget as w, Element, Length, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    // For direct message view between two people
    selected_person_1: Option<String>,
    selected_person_2: Option<String>,

    // For scene-based conversation view
    selected_scene: Option<String>,

    // View mode toggle
    view_mode: ViewMode,

    // Status for loading messages
    messages_status: MessagesStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    DirectMessage,
    Scene,
}

impl Default for ViewMode {
    fn default() -> Self {
        ViewMode::DirectMessage
    }
}

enum MessagesStatus {
    Ready,
    Loading,
    Loaded(Vec<Message>),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Message {
    // Placeholder structure for messages
    pub content: String,
    pub sender: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    ViewModeChanged(ViewMode),
    Person1Selected(String),
    Person2Selected(String),
    SceneSelected(String),
    LoadMessages,
    MessagesLoaded(Result<Vec<Message>, String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Storage {
    #[serde(default)]
    selected_person_1: Option<String>,
    #[serde(default)]
    selected_person_2: Option<String>,
    #[serde(default)]
    selected_scene: Option<String>,
    #[serde(default)]
    view_mode: ViewMode,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            selected_person_1: None,
            selected_person_2: None,
            selected_scene: None,
            view_mode: ViewMode::default(),
        }
    }
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            selected_person_1: storage.selected_person_1.clone(),
            selected_person_2: storage.selected_person_2.clone(),
            selected_scene: storage.selected_scene.clone(),
            view_mode: storage.view_mode,
            messages_status: MessagesStatus::Ready,
        }
    }

    pub fn update(&mut self, _worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::ViewModeChanged(mode) => {
                self.view_mode = mode;
                Task::none()
            }
            Msg::Person1Selected(person) => {
                self.selected_person_1 = Some(person);
                Task::none()
            }
            Msg::Person2Selected(person) => {
                self.selected_person_2 = Some(person);
                Task::none()
            }
            Msg::SceneSelected(scene) => {
                self.selected_scene = Some(scene);
                Task::none()
            }
            Msg::LoadMessages => {
                self.messages_status = MessagesStatus::Loading;
                // TODO: Implement actual message loading from database
                Task::none()
            }
            Msg::MessagesLoaded(result) => {
                self.messages_status = match result {
                    Ok(messages) => MessagesStatus::Loaded(messages),
                    Err(err) => MessagesStatus::Error(err),
                };
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<Msg> {
        let mode_selector = w::row![
            w::radio(
                "Direct Message",
                ViewMode::DirectMessage,
                Some(self.view_mode),
                Msg::ViewModeChanged
            ),
            w::radio(
                "Scene",
                ViewMode::Scene,
                Some(self.view_mode),
                Msg::ViewModeChanged
            )
        ]
        .spacing(s::S4);

        let content_view: Element<Msg> = match self.view_mode {
            ViewMode::DirectMessage => self.view_direct_message(),
            ViewMode::Scene => self.view_scene(),
        };

        w::column![
            w::text("Messages").size(24),
            mode_selector,
            content_view
        ]
        .spacing(s::S4)
        .width(Length::Fill)
        .into()
    }

    fn view_direct_message(&self) -> Element<Msg> {
        let person1_section = w::column![
            w::text("Person 1"),
            w::text_input(
                "Enter person name",
                self.selected_person_1.as_deref().unwrap_or("")
            )
            .on_input(Msg::Person1Selected),
        ]
        .spacing(s::S1);

        let person2_section = w::column![
            w::text("Person 2"),
            w::text_input(
                "Enter person name",
                self.selected_person_2.as_deref().unwrap_or("")
            )
            .on_input(Msg::Person2Selected),
        ]
        .spacing(s::S1);

        let messages_view = self.view_messages();

        w::column![person1_section, person2_section, messages_view]
            .spacing(s::S4)
            .width(Length::Fill)
            .into()
    }

    fn view_scene(&self) -> Element<Msg> {
        let scene_section = w::column![
            w::text("Scene"),
            w::text_input(
                "Enter scene name",
                self.selected_scene.as_deref().unwrap_or("")
            )
            .on_input(Msg::SceneSelected),
        ]
        .spacing(s::S1);

        let messages_view = self.view_messages();

        w::column![scene_section, messages_view]
            .spacing(s::S4)
            .width(Length::Fill)
            .into()
    }

    fn view_messages(&self) -> Element<Msg> {
        match &self.messages_status {
            MessagesStatus::Ready => {
                w::column![
                    w::text("Select people or a scene to view messages"),
                    w::button("Load Messages").on_press(Msg::LoadMessages)
                ]
                .spacing(s::S1)
                .into()
            }
            MessagesStatus::Loading => w::text("Loading messages...").into(),
            MessagesStatus::Loaded(messages) => {
                if messages.is_empty() {
                    w::text("No messages found").into()
                } else {
                    let message_list = messages.iter().fold(w::column![], |col, msg| {
                        col.push(w::text(format!("{}: {}", msg.sender, msg.content)))
                    });

                    w::scrollable(message_list)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                }
            }
            MessagesStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
        }
    }

    pub fn to_storage(&self) -> Storage {
        Storage {
            selected_person_1: self.selected_person_1.clone(),
            selected_person_2: self.selected_person_2.clone(),
            selected_scene: self.selected_scene.clone(),
            view_mode: self.view_mode,
        }
    }
}
