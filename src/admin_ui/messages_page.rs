use super::s;
use crate::capability::scene::{Scene, SceneCapability};
use crate::domain::job::send_message_to_scene::send_scene_message_and_enqueue_recipients;
use crate::domain::message::MessageSender;
use crate::domain::random_seed::RandomSeed;
use crate::domain::scene_uuid::SceneUuid;
use crate::nice_display::NiceDisplay;
use crate::worker::Worker;
use iced::{time, widget as w, Element, Length, Subscription, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

mod scene_timeline;

pub struct Model {
    // For direct message view between two people
    selected_person_1: Option<String>,
    selected_person_2: Option<String>,

    // For scene-based conversation view
    scene_name_input: String,
    scene_load_status: SceneLoadStatus,

    // Message composition
    message_input: String,
    send_status: SendStatus,

    // View mode toggle
    view_mode: ViewMode,
    auto_refresh: bool,
}

#[derive(Debug, Clone)]
pub struct LoadedSceneModel {
    pub uuid: SceneUuid,
    pub name: String,
    pub description: Option<String>,
    pub messages: MessagesStatus,
}

enum SceneLoadStatus {
    Ready,
    Loading,
    Loaded(LoadedSceneModel),
    NotFound(String), // Scene name that wasn't found
    Error(String),
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

#[derive(Debug, Clone)]
pub enum MessagesStatus {
    Loading,
    Loaded(scene_timeline::Model),
    Error(String),
}

#[derive(Debug, Clone)]
enum SendStatus {
    Ready,
    Sending,
    Sent,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum Msg {
    ViewModeChanged(ViewMode),
    Person1Selected(String),
    Person2Selected(String),
    SceneNameInputChanged(String),
    LoadScene,
    SceneLoaded(Result<Option<Scene>, String>),
    TimelineLoaded(Result<scene_timeline::Model, String>),
    MessageInputChanged(String),
    SubmitMessage,
    MessageSent(Result<(), String>),
    GotTimelineMsg(scene_timeline::Msg),
    ClickedToggleAutoRefresh,
    AutoRefreshTick,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Storage {
    #[serde(default)]
    selected_person_1: Option<String>,
    #[serde(default)]
    selected_person_2: Option<String>,
    #[serde(default)]
    scene_name_input: String,
    #[serde(default)]
    view_mode: ViewMode,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            selected_person_1: None,
            selected_person_2: None,
            scene_name_input: String::new(),
            view_mode: ViewMode::default(),
        }
    }
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            selected_person_1: storage.selected_person_1.clone(),
            selected_person_2: storage.selected_person_2.clone(),
            scene_name_input: storage.scene_name_input.clone(),
            scene_load_status: SceneLoadStatus::Ready,
            message_input: String::new(),
            send_status: SendStatus::Ready,
            view_mode: storage.view_mode,
            auto_refresh: false,
        }
    }

    pub fn update(&mut self, worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::ViewModeChanged(mode) => {
                self.view_mode = mode;
                self.send_status = SendStatus::Ready;
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
            Msg::SceneNameInputChanged(name) => {
                self.scene_name_input = name;
                self.scene_load_status = SceneLoadStatus::Ready;
                self.send_status = SendStatus::Ready;
                Task::none()
            }
            Msg::LoadScene => {
                self.scene_load_status = SceneLoadStatus::Loading;
                let scene_name = self.scene_name_input.clone();
                Task::perform(
                    async move { worker.get_scene_from_name(scene_name.clone()).await },
                    Msg::SceneLoaded,
                )
            }
            Msg::SceneLoaded(result) => match result {
                Ok(Some(scene)) => {
                    let scene_uuid = scene.uuid.clone();

                    let loaded_scene = LoadedSceneModel {
                        uuid: scene.uuid,
                        name: scene.name,
                        description: scene.description,
                        messages: MessagesStatus::Loading,
                    };

                    self.scene_load_status = SceneLoadStatus::Loaded(loaded_scene);
                    self.send_status = SendStatus::Ready;

                    Task::perform(
                        async move { scene_timeline::Model::load(&worker, scene_uuid).await },
                        Msg::TimelineLoaded,
                    )
                }
                Ok(None) => {
                    self.scene_load_status =
                        SceneLoadStatus::NotFound(self.scene_name_input.clone());
                    self.send_status = SendStatus::Ready;

                    Task::none()
                }
                Err(err) => {
                    self.scene_load_status = SceneLoadStatus::Error(err);
                    self.send_status = SendStatus::Ready;

                    Task::none()
                }
            },
            Msg::TimelineLoaded(res) => {
                if let SceneLoadStatus::Loaded(loaded_scene) = &mut self.scene_load_status {
                    match res {
                        Ok(timeline_model) => {
                            let scroll_task =
                                timeline_model.scroll_to_bottom().map(Msg::GotTimelineMsg);
                            loaded_scene.messages = MessagesStatus::Loaded(timeline_model);
                            return scroll_task;
                        }
                        Err(err) => {
                            loaded_scene.messages = MessagesStatus::Error(err);
                        }
                    }
                }
                Task::none()
            }
            Msg::MessageInputChanged(content) => {
                self.message_input = content;
                self.send_status = SendStatus::Ready;
                Task::none()
            }
            Msg::ClickedToggleAutoRefresh => {
                self.auto_refresh = !self.auto_refresh;
                if self.auto_refresh {
                    self.refresh_loaded_scene(worker)
                } else {
                    Task::none()
                }
            }
            Msg::SubmitMessage => {
                // Only allow sending if we have a loaded scene and message content
                if let SceneLoadStatus::Loaded(scene) = &self.scene_load_status {
                    if self.message_input.trim().is_empty() {
                        return Task::none();
                    }

                    let sender = MessageSender::RealWorldUser;
                    let scene_uuid = scene.uuid.clone();
                    let content = self.message_input.clone();
                    let random_seed = RandomSeed::from_u64(rand::random());
                    self.send_status = SendStatus::Sending;

                    Task::perform(
                        async move {
                            send_scene_message_and_enqueue_recipients(
                                worker.as_ref(),
                                sender,
                                scene_uuid,
                                content,
                                random_seed,
                            )
                            .await
                            .map(|_| ())
                            .map_err(|err| err.to_nice_error().to_string())
                        },
                        Msg::MessageSent,
                    )
                } else {
                    Task::none()
                }
            }
            Msg::MessageSent(result) => {
                match result {
                    Ok(_) => {
                        self.send_status = SendStatus::Sent;
                        self.message_input.clear();

                        // Reload messages after a brief moment to show the newly sent message
                        if let SceneLoadStatus::Loaded(scene) = &mut self.scene_load_status {
                            scene.messages = MessagesStatus::Loading;
                            let scene_uuid = scene.uuid.clone();
                            return Task::perform(
                                async move {
                                    scene_timeline::Model::load(&worker, scene_uuid).await
                                },
                                Msg::TimelineLoaded,
                            );
                        }
                    }
                    Err(err) => {
                        self.send_status = SendStatus::Error(err);
                    }
                }
                Task::none()
            }
            Msg::GotTimelineMsg(sub_msg) => {
                if let SceneLoadStatus::Loaded(loaded_scene) = &mut self.scene_load_status {
                    if let MessagesStatus::Loaded(timeline_model) = &mut loaded_scene.messages {
                        return timeline_model.update(sub_msg).map(Msg::GotTimelineMsg);
                    }
                }
                Task::none()
            }
            Msg::AutoRefreshTick => {
                if self.auto_refresh {
                    self.refresh_loaded_scene(worker)
                } else {
                    Task::none()
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Msg> {
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

        w::column![w::text("Messages").size(24), mode_selector, content_view]
            .spacing(s::S4)
            .width(Length::Fill)
            .into()
    }

    fn view_direct_message(&self) -> Element<'_, Msg> {
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

        // let messages_view = self.view_messages();
        let messages_view = w::text("WIP - messages view");

        w::column![person1_section, person2_section, messages_view]
            .spacing(s::S4)
            .width(Length::Fill)
            .into()
    }

    fn view_scene(&self) -> Element<'_, Msg> {
        let scene_input = w::row![
            w::text_input("Enter scene name", &self.scene_name_input)
                .on_input(Msg::SceneNameInputChanged),
            w::button("Load Scene").on_press(Msg::LoadScene),
        ]
        .spacing(s::S1);

        let scene_status: Element<'_, Msg> = match &self.scene_load_status {
            SceneLoadStatus::Ready => w::text("").into(),
            SceneLoadStatus::Loading => w::text("Loading scene...").into(),
            SceneLoadStatus::Loaded(scene) => {
                let message_composer = self.view_message_composer();
                let auto_refresh_button = self.view_auto_refresh_button();

                let description_view: Element<'_, Msg> = match &scene.description {
                    Some(desc) => w::text(desc).into(),
                    None => w::text("").into(),
                };

                w::column![
                    w::text(format!(
                        "Loaded: {} (UUID: {})",
                        scene.name,
                        scene.uuid.to_uuid()
                    )),
                    description_view,
                    auto_refresh_button,
                    view_messages(&scene.messages),
                    message_composer
                ]
                .spacing(s::S4)
                .into()
            }
            SceneLoadStatus::NotFound(name) => {
                w::text(format!("Scene '{}' not found", name)).into()
            }
            SceneLoadStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
        };

        let scene_section = w::column![w::text("Scene"), scene_input, scene_status].spacing(s::S1);

        w::column![scene_section]
            .spacing(s::S4)
            .width(Length::Fill)
            .into()
    }

    fn view_message_composer(&self) -> Element<'_, Msg> {
        let input = w::text_input("Type your message...", &self.message_input)
            .on_input(Msg::MessageInputChanged)
            .on_submit(Msg::SubmitMessage)
            .width(Length::Fill);

        let send_button = match &self.send_status {
            SendStatus::Ready | SendStatus::Sent => w::button("Send").on_press(Msg::SubmitMessage),
            SendStatus::Sending => w::button("Sending..."),
            SendStatus::Error(_) => w::button("Send").on_press(Msg::SubmitMessage),
        };

        let status_text: Element<'_, Msg> = match &self.send_status {
            SendStatus::Ready => w::text("").into(),
            SendStatus::Sending => w::text("Sending...").into(),
            SendStatus::Sent => w::text("Message sent!").into(),
            SendStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
        };

        w::column![w::row![input, send_button].spacing(s::S1), status_text]
            .spacing(s::S1)
            .into()
    }

    fn view_auto_refresh_button(&self) -> Element<'_, Msg> {
        let label = if self.auto_refresh {
            "Auto refresh: On"
        } else {
            "Auto refresh: Off"
        };

        w::button(label).on_press(Msg::ClickedToggleAutoRefresh).into()
    }

    fn refresh_loaded_scene(&mut self, worker: Arc<Worker>) -> Task<Msg> {
        if let SceneLoadStatus::Loaded(scene) = &mut self.scene_load_status {
            scene.messages = MessagesStatus::Loading;
            let scene_uuid = scene.uuid.clone();
            return Task::perform(
                async move { scene_timeline::Model::load(&worker, scene_uuid).await },
                Msg::TimelineLoaded,
            );
        }

        Task::none()
    }

    pub fn to_storage(&self) -> Storage {
        Storage {
            selected_person_1: self.selected_person_1.clone(),
            selected_person_2: self.selected_person_2.clone(),
            scene_name_input: self.scene_name_input.clone(),
            view_mode: self.view_mode,
        }
    }

    pub fn subscription(&self) -> Subscription<Msg> {
        if self.auto_refresh && self.view_mode == ViewMode::Scene {
            time::every(std::time::Duration::from_secs(2)).map(|_| Msg::AutoRefreshTick)
        } else {
            Subscription::none()
        }
    }
}

fn view_messages(messages_status: &MessagesStatus) -> Element<'_, Msg> {
    match &messages_status {
        MessagesStatus::Loading => w::text("Loading messages...").into(),
        MessagesStatus::Loaded(timeline_model) => timeline_model.view().map(Msg::GotTimelineMsg),
        MessagesStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
    }
}
