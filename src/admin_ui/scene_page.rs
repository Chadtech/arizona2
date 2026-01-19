use crate::capability::scene::{NewScene, Scene, SceneParticipant};
use crate::domain::scene_participant_uuid::SceneParticipantUuid;
use crate::domain::scene_uuid::SceneUuid;
use crate::worker::Worker;
use crate::{admin_ui::s, capability::scene::SceneCapability};
use iced::{widget as w, Element, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    new_scene_name: String,
    new_scene_description: String,
    look_up_scene: LookUpScene,
    look_up_scene_name: String,
    status: NewSceneStatus,
}

enum LookUpScene {
    Ready,
    LookingUpScene,
    LoadedScene(SceneModel),
    ErrorLookingUpScene(String),
}

struct SceneModel {
    scene_name: String,
    scene_uuid: SceneUuid,
    scene_snapshot: Option<String>,
    participants: Vec<SceneParticipant>,
    new_participant_field: String,
    new_participant_status: NewParticipantStatus,
}

enum NewParticipantStatus {
    Ready,
    AddingParticipant,
    RefreshingParticipants,
    Done,
    ErrorAddingParticipant(String),
    ErrorRefreshingParticipants(String),
}

enum NewSceneStatus {
    Ready,
    CreatingScene,
    Done,
    ErrorCreatingScene(String),
}

#[derive(Debug, Clone)]
pub struct SceneAggregate {
    scene: Scene,
    participants: Vec<SceneParticipant>,
}

impl SceneAggregate {
    async fn get(worker: Arc<Worker>, scene_name: String) -> Result<Option<Self>, String> {
        let maybe_scene = worker.get_scene_from_name(scene_name).await?;

        let scene = match maybe_scene {
            Some(scene) => scene,
            None => return Ok(None),
        };

        let participants = worker.get_scene_current_participants(&scene.uuid).await?;

        let ret = Self {
            scene,
            participants,
        };

        Ok(Some(ret))
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    NewSceneNameChanged(String),
    NewSceneDescriptionChanged(String),
    ClickedCreateScene,
    SceneCreated(Result<SceneUuid, String>),
    SceneNameChanged(String),
    ClickedLookUpScene,
    SceneLookUpMsg(SceneLookUpMsg),
    LookedUpScene(Result<Option<SceneAggregate>, String>),
}

#[derive(Debug, Clone)]
pub enum SceneLookUpMsg {
    NewParticipantFieldChanged(String),
    ClickedAddParticipant,
    AddedParticipant(Result<SceneParticipantUuid, String>),
    GotRefreshedParticipants(Result<Vec<SceneParticipant>, String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Storage {
    #[serde(default)]
    new_scene_name: String,
    #[serde(default)]
    new_scene_description: String,
    #[serde(default)]
    look_up_scene_name: String,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            new_scene_name: String::new(),
            new_scene_description: String::new(),
            look_up_scene_name: String::new(),
        }
    }
}

impl SceneModel {
    fn init(scene_agg: SceneAggregate) -> Self {
        let scene = scene_agg.scene;

        Self {
            scene_name: scene.name,
            scene_uuid: scene.uuid,
            scene_snapshot: scene.description,
            participants: scene_agg.participants,
            new_participant_field: "".to_string(),
            new_participant_status: NewParticipantStatus::Ready,
        }
    }

    fn update(&mut self, worker: Arc<Worker>, msg: SceneLookUpMsg) -> Task<SceneLookUpMsg> {
        match msg {
            SceneLookUpMsg::NewParticipantFieldChanged(field) => {
                self.new_participant_field = field;
                Task::none()
            }
            SceneLookUpMsg::ClickedAddParticipant => match self.new_participant_status {
                NewParticipantStatus::Ready => {
                    self.new_participant_status = NewParticipantStatus::AddingParticipant;
                    let scene_uuid = self.scene_uuid.clone();
                    let person_name = self.new_participant_field.clone();
                    Task::perform(
                        async move {
                            worker
                                .add_person_to_scene(scene_uuid, person_name.into())
                                .await
                        },
                        SceneLookUpMsg::AddedParticipant,
                    )
                }
                _ => Task::none(),
            },
            SceneLookUpMsg::AddedParticipant(result) => match result {
                Ok(_) => {
                    self.new_participant_status = NewParticipantStatus::RefreshingParticipants;

                    let scene_uuid = self.scene_uuid.clone();

                    Task::perform(
                        async move { worker.get_scene_current_participants(&scene_uuid).await },
                        SceneLookUpMsg::GotRefreshedParticipants,
                    )
                }
                Err(err) => {
                    self.new_participant_status = NewParticipantStatus::ErrorAddingParticipant(err);
                    Task::none()
                }
            },
            SceneLookUpMsg::GotRefreshedParticipants(result) => {
                self.new_participant_status = match &result {
                    Ok(_) => NewParticipantStatus::Done,
                    Err(err) => NewParticipantStatus::ErrorRefreshingParticipants(err.clone()),
                };
                if let Ok(participants) = result {
                    self.participants = participants;
                }
                Task::none()
            }
        }
    }
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            new_scene_name: storage.new_scene_name.clone(),
            new_scene_description: storage.new_scene_description.clone(),
            look_up_scene_name: storage.look_up_scene_name.clone(),
            look_up_scene: LookUpScene::Ready,
            status: NewSceneStatus::Ready,
        }
    }
    pub fn to_storage(&self) -> Storage {
        Storage {
            new_scene_name: self.new_scene_name.clone(),
            new_scene_description: self.new_scene_description.clone(),
            look_up_scene_name: self.look_up_scene_name.clone(),
        }
    }

    pub fn view(&self) -> Element<'_, Msg> {
        w::column![
            new_scene_view(&self),
            w::text("Look Up Scene"),
            w::row![
                w::text_input("Scene Name", self.look_up_scene_name.as_str())
                    .on_input(Msg::SceneNameChanged),
                w::button("Look Up Scene").on_press(Msg::ClickedLookUpScene),
            ]
            .spacing(s::S4),
            scene_look_up_view(&self.look_up_scene).map(Msg::SceneLookUpMsg),
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
            Msg::SceneLookUpMsg(sub_msg) => {
                if let LookUpScene::LoadedScene(scene_model) = &mut self.look_up_scene {
                    let task = scene_model.update(worker, sub_msg);
                    task.map(Msg::SceneLookUpMsg)
                } else {
                    Task::none()
                }
            }
            Msg::SceneNameChanged(field) => {
                self.look_up_scene_name = field;
                Task::none()
            }
            Msg::ClickedLookUpScene => match self.look_up_scene {
                LookUpScene::Ready => {
                    self.look_up_scene = LookUpScene::LookingUpScene;
                    let scene_name = self.look_up_scene_name.clone();
                    Task::perform(
                        async move { SceneAggregate::get(worker, scene_name).await },
                        Msg::LookedUpScene,
                    )
                }
                _ => Task::none(),
            },
            Msg::LookedUpScene(result) => {
                self.look_up_scene = match result {
                    Ok(Some(scene_agg)) => LookUpScene::LoadedScene(SceneModel::init(scene_agg)),
                    Ok(None) => LookUpScene::ErrorLookingUpScene("Scene not found".to_string()),
                    Err(err) => LookUpScene::ErrorLookingUpScene(err),
                };
                Task::none()
            }
        }
    }
}

fn scene_look_up_view(scene_look_up: &LookUpScene) -> Element<'_, SceneLookUpMsg> {
    let look_up_view = match scene_look_up {
        LookUpScene::Ready => w::text("Ready to look up scene").into(),
        LookUpScene::LookingUpScene => w::text("Looking up scene...").into(),
        LookUpScene::LoadedScene(scene_model) => scene_loaded_view(scene_model),
        LookUpScene::ErrorLookingUpScene(err) => {
            w::text(format!("Error looking up scene: {}", err)).into()
        }
    };

    w::column![look_up_view].spacing(s::S4).into()
}

fn scene_loaded_view(scene_model: &SceneModel) -> Element<'_, SceneLookUpMsg> {
    let description = scene_model
        .scene_snapshot
        .clone()
        .unwrap_or("No description available".to_string());

    let participants: Element<SceneLookUpMsg> = if scene_model.participants.is_empty() {
        w::text("No participants").into()
    } else {
        w::column(
            scene_model
                .participants
                .iter()
                .map(|p| w::text(p.person_name.as_str()).into())
                .collect::<Vec<_>>(),
        )
        .into()
    };

    let new_participant_status: Element<SceneLookUpMsg> = match &scene_model.new_participant_status
    {
        NewParticipantStatus::Ready => w::text("Ready").into(),
        NewParticipantStatus::AddingParticipant => w::text("Adding participant...").into(),
        NewParticipantStatus::RefreshingParticipants => {
            w::text("Refreshing participants...").into()
        }
        NewParticipantStatus::Done => w::text("Done!").into(),
        NewParticipantStatus::ErrorAddingParticipant(err) => {
            w::text(format!("Error adding participant: {}", err)).into()
        }
        NewParticipantStatus::ErrorRefreshingParticipants(err) => {
            w::text(format!("Error refreshing participants: {}", err)).into()
        }
    };

    w::column![
        w::text("Scene Name"),
        w::text(&scene_model.scene_name),
        w::text("Description"),
        w::text(description),
        w::text("Participants"),
        participants,
        w::text_input(
            "Participant Name",
            scene_model.new_participant_field.as_str()
        )
        .on_input(SceneLookUpMsg::NewParticipantFieldChanged),
        w::button("Add Participant").on_press(SceneLookUpMsg::ClickedAddParticipant),
        new_participant_status
    ]
    .spacing(s::S4)
    .into()
}

fn new_scene_view(model: &Model) -> Element<'_, Msg> {
    w::column![
        w::text("Scene Name"),
        w::text_input("", &model.new_scene_name).on_input(Msg::NewSceneNameChanged),
        w::text("Scene Description"),
        w::text_input("", &model.new_scene_description).on_input(Msg::NewSceneDescriptionChanged),
        w::button("Create Scene").on_press(Msg::ClickedCreateScene),
        scene_creation_status_view(&model.status),
    ]
    .spacing(s::S4)
    .into()
}

fn scene_creation_status_view(status: &NewSceneStatus) -> Element<'_, Msg> {
    match status {
        NewSceneStatus::Ready => w::text("Ready").into(),
        NewSceneStatus::CreatingScene => w::text("Creating scene...").into(),
        NewSceneStatus::Done => w::text("Done!").into(),
        NewSceneStatus::ErrorCreatingScene(err) => {
            w::text(format!("Error creating scene: {}", err)).into()
        }
    }
}
