use crate::admin_ui::{create_new_identity, s};
use crate::capability::person_identity::NewPersonIdentity;
use crate::domain::person_identity_uuid::PersonIdentityUuid;
use crate::worker::Worker;
use iced::{widget as w, Element, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    name_field: String,
    identity_field: String,
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
    ClickedAddIdentity,
    IdentityCreated(Result<PersonIdentityUuid, String>),
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            identity_field: storage.identity_field.clone(),
            name_field: storage.name_field.clone(),
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
            Msg::ClickedAddIdentity => {
                let new_identity = NewPersonIdentity {
                    identity: self.identity_field.clone(),
                    person_name: self.name_field.clone(),
                };
                Task::perform(
                    async move { create_new_identity(&worker, new_identity).await },
                    Msg::IdentityCreated,
                )
            }
            Msg::IdentityCreated(result) => {
                dbg!(&result);
                Task::none()
            }
        }
    }
    pub fn view(&self) -> Element<Msg> {
        w::column![
            w::text("Person Name"),
            w::text_input("Person Name", &self.name_field).on_input(Msg::NameFieldChanged),
            w::text("New Identity"),
            w::text_input("New Identity", &self.identity_field).on_input(Msg::IdentityFieldChanged),
            w::button("Add Identity").on_press(Msg::ClickedAddIdentity),
        ]
        .spacing(s::S4)
        .into()
    }
}
