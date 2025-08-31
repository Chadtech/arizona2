use crate::admin_ui::style::S4;
use crate::capability::state_of_mind::NewStateOfMind;
use crate::worker::Worker;
use crate::{
    capability::state_of_mind::StateOfMindCapability, domain::state_of_mind_uuid::StateOfMindUuid,
};
use iced::widget as w;
use iced::{Element, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    name_field: String,
    state_of_mind_field: String,
    status: Status,
}

enum Status {
    Ready,
    CreatingStateOfMind,
    Done,
    FailedCreatingStateOfMind(String),
}

#[derive(Debug, Clone)]
pub enum Msg {
    NameFieldChanged(String),
    StateOfMindFieldChanged(String),
    ClickedCreateStateOfMind,
    CreatedStateOfMind(Result<StateOfMindUuid, String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Storage {
    #[serde(default)]
    name_field: String,
    #[serde(default)]
    state_of_mind_field: String,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            name_field: String::new(),
            state_of_mind_field: String::new(),
        }
    }
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            name_field: storage.name_field.clone(),
            state_of_mind_field: storage.state_of_mind_field.clone(),
            status: Status::Ready,
        }
    }

    pub fn view(&self) -> Element<Msg> {
        w::column![
            w::text("Person Name"),
            w::text_input("", &self.name_field).on_input(Msg::NameFieldChanged),
            w::text("State of Mind"),
            w::text_input("", &self.state_of_mind_field).on_input(Msg::StateOfMindFieldChanged),
            w::button("Create State of Mind").on_press(Msg::ClickedCreateStateOfMind),
            status_view(&self.status)
        ]
        .spacing(S4)
        .into()
    }

    pub fn to_storage(&self) -> Storage {
        Storage {
            name_field: self.name_field.clone(),
            state_of_mind_field: self.state_of_mind_field.clone(),
        }
    }

    pub fn update(&mut self, worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::NameFieldChanged(name) => {
                self.name_field = name;
                Task::none()
            }
            Msg::StateOfMindFieldChanged(state_of_mind) => {
                self.state_of_mind_field = state_of_mind;
                Task::none()
            }
            Msg::ClickedCreateStateOfMind => {
                self.status = Status::CreatingStateOfMind;

                let new_state_of_mind = NewStateOfMind {
                    uuid: StateOfMindUuid::new(),
                    person_name: crate::domain::person_name::PersonName::from_string(
                        self.name_field.clone(),
                    ),
                    state_of_mind: self.state_of_mind_field.clone(),
                };

                Task::perform(
                    async move { worker.create_state_of_mind(new_state_of_mind).await },
                    Msg::CreatedStateOfMind,
                )
            }
            Msg::CreatedStateOfMind(result) => {
                self.status = match result {
                    Ok(_) => Status::Done,
                    Err(err) => Status::FailedCreatingStateOfMind(err),
                };
                Task::none()
            }
        }
    }
}

fn status_view(status: &Status) -> Element<Msg> {
    match status {
        Status::Ready => w::text("Ready").into(),
        Status::CreatingStateOfMind => w::text("Creating State of Mind...").into(),
        Status::Done => w::text("Done!").into(),
        Status::FailedCreatingStateOfMind(err) => w::text(format!("Error: {}", err)).into(),
    }
}
