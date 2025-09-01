use crate::capability::scene::NewScene;
use crate::domain::scene_uuid::SceneUuid;
use crate::worker::Worker;
use crate::{admin_ui::s, capability::scene::SceneCapability};
use iced::{widget as w, Element, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    new_scene_name: String,
    new_scene_description: String,
    status: NewSceneStatus,
}

enum NewSceneStatus {
    Ready,
    CreatingScene,
    AddingDescription,
    Done,
    ErrorCreatingScene(String),
}

#[derive(Debug, Clone)]
pub enum Msg {
    NewSceneNameChanged(String),
    NewSceneDescriptionChanged(String),
    ClickedCreateScene,
    SceneCreated(Result<SceneUuid, String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Storage {
    #[serde(default)]
    new_scene_name: String,
    #[serde(default)]
    new_scene_description: String,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            new_scene_name: String::new(),
            new_scene_description: String::new(),
        }
    }
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            new_scene_name: storage.new_scene_name.clone(),
            new_scene_description: storage.new_scene_description.clone(),
            status: NewSceneStatus::Ready,
        }
    }
    pub fn to_storage(&self) -> Storage {
        Storage {
            new_scene_name: self.new_scene_name.clone(),
            new_scene_description: self.new_scene_description.clone(),
        }
    }

    pub fn view(&self) -> Element<Msg> {
        w::column![
            w::text("Scene Name"),
            w::text_input("", &self.new_scene_name).on_input(Msg::NewSceneNameChanged),
            w::text("Scene Description"),
            w::text_input("", &self.new_scene_description)
                .on_input(Msg::NewSceneDescriptionChanged),
            w::button("Create Scene").on_press(Msg::ClickedCreateScene),
            status_view(&self.status)
        ]
        .spacing(s::S4)
        .into()
    }

    pub fn update(&mut self, worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::NewSceneNameChanged(name) => {
                self.new_scene_name = name;
                Task::none()
            }
            Msg::NewSceneDescriptionChanged(description) => {
                self.new_scene_description = description;
                Task::none()
            }
            Msg::ClickedCreateScene => match self.status {
                NewSceneStatus::Ready => {
                    self.status = NewSceneStatus::CreatingScene;

                    let new_scene = NewScene {
                        name: self.new_scene_name.clone(),
                        description: self.new_scene_description.clone(),
                    };
                    Task::perform(
                        async move { worker.create_scene(new_scene).await },
                        Msg::SceneCreated,
                    )
                }
                _ => Task::none(),
            },
            Msg::SceneCreated(result) => {
                self.status = match result {
                    Ok(_scene_uuid) => NewSceneStatus::Done,
                    Err(err) => NewSceneStatus::ErrorCreatingScene(err),
                };
                Task::none()
            }
        }
    }
}

fn status_view(status: &NewSceneStatus) -> Element<Msg> {
    match status {
        NewSceneStatus::Ready => w::text("Ready").into(),
        NewSceneStatus::CreatingScene => w::text("Creating scene...").into(),
        NewSceneStatus::AddingDescription => w::text("Adding description...").into(),
        NewSceneStatus::Done => w::text("Done!").into(),
        NewSceneStatus::ErrorCreatingScene(err) => {
            w::text(format!("Error creating scene: {}", err)).into()
        }
    }
}
