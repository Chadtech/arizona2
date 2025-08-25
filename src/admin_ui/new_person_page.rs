use crate::admin_ui::s;
use crate::capability::person::{NewPerson, PersonCapability};
use crate::capability::person_identity::{NewPersonIdentity, PersonIdentityCapability};
use crate::domain::person_identity_uuid::PersonIdentityUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::worker::Worker;
use iced::{widget as w, Element, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    name_field: String,
    identity_field: String,
    status: Status,
}

enum Status {
    Ready,
    CreatingPerson { identity: String, name: String },
    CreatingIdentity,
    Done,
    ErrorCreatingPerson(String),
    ErrorCreatingIdentity(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Storage {
    #[serde(default)]
    identity_field: String,
    #[serde(default)]
    name_field: String,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            identity_field: String::new(),
            name_field: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    IdentityFieldChanged(String),
    NameFieldChanged(String),
    ClickedCreatePerson,
    PersonCreated(Result<PersonUuid, String>),
    IdentityCreated(Result<PersonIdentityUuid, String>),
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            identity_field: storage.identity_field.clone(),
            name_field: storage.name_field.clone(),
            status: Status::Ready,
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
                Task::none()
            }
            Msg::NameFieldChanged(value) => {
                self.name_field = value;
                Task::none()
            }
            Msg::ClickedCreatePerson => {
                // match self.status {
                //     Status::Ready => {
                self.status = Status::CreatingPerson {
                    identity: self.identity_field.clone(),
                    name: self.name_field.clone(),
                };

                let new_person = NewPerson {
                    person_name: self.name_field.clone(),
                    person_uuid: PersonUuid::new(),
                };

                Task::perform(
                    async move { create_new_person(&worker, new_person).await },
                    Msg::PersonCreated,
                )
                //     }
                //     _ => Task::none(),
                // }
            }
            Msg::PersonCreated(result) => match result {
                Ok(_) => {
                    self.status = Status::CreatingIdentity;

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
                Err(err) => {
                    self.status = Status::ErrorCreatingPerson(err);
                    Task::none()
                }
            },
            Msg::IdentityCreated(result) => {
                self.status = match result {
                    Ok(_) => Status::Done,
                    Err(err) => Status::ErrorCreatingIdentity(err),
                };
                Task::none()
            }
        }
    }
    pub fn view(&self) -> Element<Msg> {
        w::column![
            w::text("Person Name"),
            w::text_input("", &self.name_field).on_input(Msg::NameFieldChanged),
            w::text("Identity"),
            w::text_input("", &self.identity_field).on_input(Msg::IdentityFieldChanged),
            w::button("Create Person").on_press(Msg::ClickedCreatePerson),
            status_view(&self.status),
        ]
        .spacing(s::S4)
        .into()
    }
}

fn status_view(status: &Status) -> Element<Msg> {
    match status {
        Status::Ready => w::text("Ready").into(),
        Status::CreatingPerson { .. } => w::text("Creating person...").into(),
        Status::CreatingIdentity => w::text("Creating identity...").into(),
        Status::Done => w::text("Done!").into(),
        Status::ErrorCreatingPerson(err) => {
            w::text(format!("Error creating person: {}", err)).into()
        }
        Status::ErrorCreatingIdentity(err) => {
            w::text(format!("Error creating identity: {}", err)).into()
        }
    }
}

async fn create_new_person(worker: &Worker, new_person: NewPerson) -> Result<PersonUuid, String> {
    worker.create_person(new_person).await
}

async fn create_new_identity(
    worker: &Worker,
    new_identity: NewPersonIdentity,
) -> Result<PersonIdentityUuid, String> {
    worker.create_person_identity(new_identity).await
}
