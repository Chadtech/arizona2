use crate::admin_ui::s;
use crate::capability::person::PersonCapability;
use crate::capability::person_identity::{NewPersonIdentity, PersonIdentityCapability};
use crate::domain::person_identity_uuid::PersonIdentityUuid;
use crate::domain::person_name::PersonName;
use crate::worker::Worker;
use iced::{widget as w, Element, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    name_field: String,
    identity_field: String,
    load_status: LoadStatus,
    save_status: SaveStatus,
}

enum LoadStatus {
    Ready,
    Loading,
    LoadedSome,
    LoadedNone,
    Error(String),
}

enum SaveStatus {
    Ready,
    Saving,
    Done,
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Storage {
    #[serde(default)]
    identity_field: String,
    #[serde(default)]
    name_field: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    IdentityFieldChanged(String),
    NameFieldChanged(String),
    ClickedLoadIdentity,
    ClickedSaveIdentity,
    IdentityLoaded(Result<Option<String>, String>),
    IdentityCreated(Result<PersonIdentityUuid, String>),
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            identity_field: storage.identity_field.clone(),
            name_field: storage.name_field.clone(),
            load_status: LoadStatus::Ready,
            save_status: SaveStatus::Ready,
        }
    }
    pub fn to_storage(&self) -> Storage {
        Storage {
            identity_field: self.identity_field.clone(),
            name_field: self.name_field.clone(),
        }
    }

    pub fn update(&mut self, worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::IdentityFieldChanged(value) => {
                self.identity_field = value;
                self.save_status = SaveStatus::Ready;
                Task::none()
            }
            Msg::NameFieldChanged(value) => {
                self.name_field = value;
                self.load_status = LoadStatus::Ready;
                self.save_status = SaveStatus::Ready;
                Task::none()
            }
            Msg::ClickedLoadIdentity => {
                if self.name_field.trim().is_empty() {
                    self.load_status = LoadStatus::Error("Person name is required".to_string());
                    return Task::none();
                }

                self.load_status = LoadStatus::Loading;
                let person_name = self.name_field.clone();
                Task::perform(
                    async move { load_identity(&worker, person_name).await },
                    Msg::IdentityLoaded,
                )
            }
            Msg::IdentityLoaded(result) => {
                match result {
                    Ok(Some(identity)) => {
                        self.identity_field = identity;
                        self.load_status = LoadStatus::LoadedSome;
                    }
                    Ok(None) => {
                        self.identity_field.clear();
                        self.load_status = LoadStatus::LoadedNone;
                    }
                    Err(err) => {
                        self.load_status = LoadStatus::Error(err);
                    }
                }
                Task::none()
            }
            Msg::ClickedSaveIdentity => {
                if self.name_field.trim().is_empty() {
                    self.save_status = SaveStatus::Error("Person name is required".to_string());
                    return Task::none();
                }
                if self.identity_field.trim().is_empty() {
                    self.save_status = SaveStatus::Error("Identity cannot be empty".to_string());
                    return Task::none();
                }

                self.save_status = SaveStatus::Saving;

                let new_identity = NewPersonIdentity {
                    person_identity_uuid: PersonIdentityUuid::new(),
                    identity: self.identity_field.clone(),
                    person_name: self.name_field.clone(),
                };

                Task::perform(
                    async move { create_new_identity(&worker, new_identity).await },
                    Msg::IdentityCreated,
                )
            }
            Msg::IdentityCreated(result) => {
                self.save_status = match result {
                    Ok(_) => SaveStatus::Done,
                    Err(err) => SaveStatus::Error(err),
                };
                Task::none()
            }
        }
    }
    pub fn view(&self) -> Element<'_, Msg> {
        w::column![
            w::text("Person Name"),
            w::row![
                w::text_input("", &self.name_field).on_input(Msg::NameFieldChanged),
                w::button("Load Identity").on_press(Msg::ClickedLoadIdentity),
            ]
            .spacing(s::S1),
            load_status_view(&self.load_status),
            w::text("Identity"),
            w::text_input("", &self.identity_field).on_input(Msg::IdentityFieldChanged),
            w::button("Save Identity").on_press(Msg::ClickedSaveIdentity),
            w::text("Saving creates a new identity entry for the person."),
            save_status_view(&self.save_status),
        ]
        .spacing(s::S4)
        .into()
    }
}

fn load_status_view(status: &LoadStatus) -> Element<'_, Msg> {
    match status {
        LoadStatus::Ready => w::text("Ready").into(),
        LoadStatus::Loading => w::text("Loading identity...").into(),
        LoadStatus::LoadedSome => w::text("Loaded identity").into(),
        LoadStatus::LoadedNone => w::text("No identity found").into(),
        LoadStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
    }
}

fn save_status_view(status: &SaveStatus) -> Element<'_, Msg> {
    match status {
        SaveStatus::Ready => w::text("Ready").into(),
        SaveStatus::Saving => w::text("Saving identity...").into(),
        SaveStatus::Done => w::text("Identity saved").into(),
        SaveStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
    }
}

async fn load_identity(worker: &Worker, person_name: String) -> Result<Option<String>, String> {
    let person_uuid = worker
        .get_person_uuid_by_name(PersonName::from_string(person_name))
        .await?;
    worker.get_person_identity(&person_uuid).await
}

async fn create_new_identity(
    worker: &Worker,
    new_identity: NewPersonIdentity,
) -> Result<PersonIdentityUuid, String> {
    worker.create_person_identity(new_identity).await
}
