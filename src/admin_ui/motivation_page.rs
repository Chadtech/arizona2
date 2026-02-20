use crate::admin_ui::s;
use crate::capability::motivation::{MotivationCapability, NewMotivation};
use crate::capability::person::PersonCapability;
use crate::domain::motivation::Motivation;
use crate::domain::motivation_uuid::MotivationUuid;
use crate::domain::person_name::PersonName;
use crate::domain::person_uuid::PersonUuid;
use crate::worker::Worker;
use iced::{widget as w, Element, Task};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub struct Model {
    person_name_input: String,
    load_status: LoadStatus,
    create_state: Option<CreateState>,
    delete_status: HashMap<MotivationUuid, DeleteStatus>,
}

enum LoadStatus {
    Ready,
    Loading,
    Loaded { person_uuid: PersonUuid, motivations: Vec<Motivation> },
    Error(String),
}

enum CreateStatus {
    Ready,
    Creating,
    Done,
    Error(String),
}

enum DeleteStatus {
    Ready,
    Deleting,
    Done,
    Error(String),
}

struct CreateState {
    content_input: String,
    priority_input: String,
    status: CreateStatus,
}

#[derive(Debug, Clone)]
pub enum Msg {
    PersonNameChanged(String),
    MotivationContentChanged(String),
    MotivationPriorityChanged(String),
    ClickedLoadMotivations,
    MotivationsLoaded(Result<(PersonUuid, Vec<Motivation>), String>),
    ClickedCreateMotivation,
    MotivationCreated(Result<MotivationUuid, String>),
    ClickedDeleteMotivation(MotivationUuid),
    MotivationDeleted(Result<MotivationUuid, String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Storage {
    #[serde(default)]
    person_name_input: String,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            person_name_input: String::new(),
        }
    }
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            person_name_input: storage.person_name_input.clone(),
            load_status: LoadStatus::Ready,
            create_state: None,
            delete_status: HashMap::new(),
        }
    }

    pub fn to_storage(&self) -> Storage {
        Storage {
            person_name_input: self.person_name_input.clone(),
        }
    }

    pub fn update(&mut self, worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::PersonNameChanged(value) => {
                self.person_name_input = value;
                self.load_status = LoadStatus::Ready;
                self.create_state = None;
                self.delete_status.clear();
                Task::none()
            }
            Msg::MotivationContentChanged(value) => {
                if let Some(create_state) = &mut self.create_state {
                    create_state.content_input = value;
                }
                Task::none()
            }
            Msg::MotivationPriorityChanged(value) => {
                if let Some(create_state) = &mut self.create_state {
                    create_state.priority_input = value;
                }
                Task::none()
            }
            Msg::ClickedLoadMotivations => {
                let person_name = self.person_name_input.clone();
                if person_name.trim().is_empty() {
                    self.load_status =
                        LoadStatus::Error("Person name cannot be empty".to_string());
                    self.create_state = None;
                    return Task::none();
                }

                self.load_status = LoadStatus::Loading;
                self.create_state = None;
                self.delete_status.clear();
                Task::perform(
                    async move { load_motivations(&worker, person_name).await },
                    Msg::MotivationsLoaded,
                )
            }
            Msg::MotivationsLoaded(result) => {
                self.load_status = match result {
                    Ok((person_uuid, motivations)) => {
                        if self.create_state.is_none() {
                            self.create_state = Some(CreateState {
                                content_input: String::new(),
                                priority_input: String::new(),
                                status: CreateStatus::Ready,
                            });
                        }
                        self.delete_status.retain(|motivation_uuid, _| {
                            motivations
                                .iter()
                                .any(|motivation| &motivation.uuid == motivation_uuid)
                        });
                        LoadStatus::Loaded { person_uuid, motivations }
                    }
                    Err(err) => {
                        self.create_state = None;
                        self.delete_status.clear();
                        LoadStatus::Error(err)
                    }
                };
                Task::none()
            }
            Msg::ClickedCreateMotivation => match &self.load_status {
                LoadStatus::Loaded { person_uuid, .. } => {
                    let create_state = match &mut self.create_state {
                        Some(state) => state,
                        None => {
                            return Task::none();
                        }
                    };

                    let priority = match create_state.priority_input.trim().parse::<i32>() {
                        Ok(value) => value,
                        Err(_) => {
                            create_state.status = CreateStatus::Error(
                                "Priority must be a number".to_string(),
                            );
                            return Task::none();
                        }
                    };

                    let content = create_state.content_input.trim();
                    if content.is_empty() {
                        create_state.status =
                            CreateStatus::Error("Motivation content cannot be empty".to_string());
                        return Task::none();
                    }

                    create_state.status = CreateStatus::Creating;
                    let person_uuid = person_uuid.clone();
                    let content = content.to_string();
                    Task::perform(
                        async move {
                            create_motivation(&worker, person_uuid, content, priority).await
                        },
                        Msg::MotivationCreated,
                    )
                }
                _ => {
                    Task::none()
                }
            },
            Msg::MotivationCreated(result) => match result {
                Ok(_) => {
                    if let Some(create_state) = &mut self.create_state {
                        create_state.status = CreateStatus::Done;
                        create_state.content_input.clear();
                    }

                    let person_name = self.person_name_input.clone();
                    self.load_status = LoadStatus::Loading;
                    Task::perform(
                        async move { load_motivations(&worker, person_name).await },
                        Msg::MotivationsLoaded,
                    )
                }
                Err(err) => {
                    if let Some(create_state) = &mut self.create_state {
                        create_state.status = CreateStatus::Error(err);
                    }
                    Task::none()
                }
            },
            Msg::ClickedDeleteMotivation(motivation_uuid) => {
                self.delete_status
                    .insert(motivation_uuid.clone(), DeleteStatus::Deleting);
                Task::perform(
                    async move { delete_motivation(&worker, motivation_uuid).await },
                    Msg::MotivationDeleted,
                )
            }
            Msg::MotivationDeleted(result) => match result {
                Ok(motivation_uuid) => {
                    self.delete_status
                        .insert(motivation_uuid, DeleteStatus::Done);

                    let person_name = self.person_name_input.clone();
                    self.load_status = LoadStatus::Loading;
                    Task::perform(
                        async move { load_motivations(&worker, person_name).await },
                        Msg::MotivationsLoaded,
                    )
                }
                Err(err) => {
                    self.load_status = LoadStatus::Error(err);
                    Task::none()
                }
            },
        }
    }

    pub fn view(&self) -> Element<'_, Msg> {
        let load_section = w::column![
            w::text("Person Name"),
            w::row![
                w::text_input("Enter person name", &self.person_name_input)
                    .on_input(Msg::PersonNameChanged),
                w::button("Load Motivations").on_press(Msg::ClickedLoadMotivations),
            ]
            .spacing(s::S1),
            load_status_view(&self.load_status),
        ]
        .spacing(s::S2);

        let motivations_list = motivations_view(&self.load_status, &self.delete_status);

        let create_section: Element<'_, Msg> = match &self.create_state {
            Some(create_state) => w::column![
                w::text("New Motivation"),
                w::text_input("Motivation content", &create_state.content_input)
                    .on_input(Msg::MotivationContentChanged),
                w::text_input("Priority (integer)", &create_state.priority_input)
                    .on_input(Msg::MotivationPriorityChanged),
                w::button("Create Motivation").on_press(Msg::ClickedCreateMotivation),
                create_status_view(&create_state.status),
            ]
            .spacing(s::S2)
            .into(),
            _ => w::column![
                w::text("New Motivation"),
                w::text("Load a person to create motivations."),
            ]
            .spacing(s::S1)
            .into(),
        };

        w::column![w::text("Motivations"), load_section, motivations_list, create_section]
            .spacing(s::S4)
            .into()
    }
}

fn load_status_view(status: &LoadStatus) -> Element<'_, Msg> {
    match status {
        LoadStatus::Ready => w::text("Ready").into(),
        LoadStatus::Loading => w::text("Loading motivations...").into(),
        LoadStatus::Loaded { motivations, .. } => {
            w::text(format!("Loaded {} motivations", motivations.len())).into()
        }
        LoadStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
    }
}

fn create_status_view(status: &CreateStatus) -> Element<'_, Msg> {
    match status {
        CreateStatus::Ready => w::text("Ready").into(),
        CreateStatus::Creating => w::text("Creating motivation...").into(),
        CreateStatus::Done => w::text("Motivation created").into(),
        CreateStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
    }
}

fn motivations_view<'a>(
    status: &'a LoadStatus,
    delete_status: &'a HashMap<MotivationUuid, DeleteStatus>,
) -> Element<'a, Msg> {
    match status {
        LoadStatus::Loaded { motivations, .. } => {
            if motivations.is_empty() {
                return w::text("No motivations found").into();
            }

            let mut col = w::column![];
            for motivation in motivations {
                let created_at = motivation.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
                let ended_at = match motivation.ended_at {
                    Some(time) => time.format("%Y-%m-%d %H:%M:%S").to_string(),
                    None => "active".to_string(),
                };

                let delete_button = match delete_status.get(&motivation.uuid) {
                    Some(DeleteStatus::Deleting) => w::button("Deleting..."),
                    Some(DeleteStatus::Done) => w::button("Deleted"),
                    Some(DeleteStatus::Error(_)) => {
                        w::button("Delete")
                            .on_press(Msg::ClickedDeleteMotivation(motivation.uuid.clone()))
                    }
                    _ => w::button("Delete")
                        .on_press(Msg::ClickedDeleteMotivation(motivation.uuid.clone())),
                };

                let delete_error: Element<'_, Msg> = match delete_status.get(&motivation.uuid) {
                    Some(DeleteStatus::Error(err)) => {
                        w::text(format!("Error: {}", err)).into()
                    }
                    _ => w::text("").into(),
                };

                col = col.push(
                    w::column![
                        w::text(format!("Priority: {}", motivation.priority)).size(s::S3),
                        w::text(format!("Created: {}", created_at)).size(s::S3),
                        w::text(format!("Ended: {}", ended_at)).size(s::S3),
                        w::text(&motivation.content),
                        delete_button,
                        delete_error,
                        w::horizontal_rule(1),
                    ]
                    .spacing(s::S1),
                );
            }

            col.spacing(s::S2).into()
        }
        LoadStatus::Loading => w::text("Loading motivations...").into(),
        LoadStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
        _ => w::text("").into(),
    }
}

async fn load_motivations(
    worker: &Worker,
    person_name: String,
) -> Result<(PersonUuid, Vec<Motivation>), String> {
    let person_uuid = worker
        .get_person_uuid_by_name(PersonName::from_string(person_name))
        .await?;
    let motivations = worker.get_motivations_for_person(person_uuid.clone()).await?;
    Ok((person_uuid, motivations))
}

async fn create_motivation(
    worker: &Worker,
    person_uuid: PersonUuid,
    content: String,
    priority: i32,
) -> Result<MotivationUuid, String> {
    let new_motivation = NewMotivation {
        person_uuid,
        content,
        priority,
    };

    worker.create_motivation(new_motivation).await
}

async fn delete_motivation(
    worker: &Worker,
    motivation_uuid: MotivationUuid,
) -> Result<MotivationUuid, String> {
    worker
        .delete_motivation(motivation_uuid.clone())
        .await
        .map_err(|err| format!("Error deleting motivation: {}", err))?;
    Ok(motivation_uuid)
}
