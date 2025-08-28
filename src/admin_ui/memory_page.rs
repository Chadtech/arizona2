use crate::capability::memory::MemoryCapability;
use crate::domain::memory_uuid::MemoryUuid;
use crate::domain::person_name::PersonName;
use crate::worker::Worker;
use crate::{admin_ui::s, capability::memory::NewMemory};
use iced::{widget as w, Element, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    name_field: String,
    memory_field: String,
    status: Status,
}

enum Status {
    Ready,
    CreatingMemory,
    Done,
    FailedCreatingMemory(String),
}

#[derive(Debug, Clone)]
pub enum Msg {
    NameFieldChanged(String),
    MemoryFieldChanged(String),
    ClickedCreateMemory,
    CreatedMemory(Result<MemoryUuid, String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Storage {
    #[serde(default)]
    name_field: String,
    #[serde(default)]
    memory_field: String,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            name_field: String::new(),
            memory_field: String::new(),
        }
    }
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            name_field: storage.name_field.clone(),
            memory_field: storage.memory_field.clone(),
            status: Status::Ready,
        }
    }

    pub fn view(&self) -> Element<Msg> {
        w::column![
            w::text("Person Name"),
            w::text_input("", &self.name_field).on_input(Msg::NameFieldChanged),
            w::text("Memory"),
            w::text_input("", &self.memory_field).on_input(Msg::MemoryFieldChanged),
            w::button("Create Memory").on_press(Msg::ClickedCreateMemory),
            status_view(&self.status)
        ]
        .spacing(s::S4)
        .into()
    }

    pub fn to_storage(&self) -> Storage {
        Storage {
            name_field: self.name_field.clone(),
            memory_field: self.memory_field.clone(),
        }
    }

    pub fn update(&mut self, worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::NameFieldChanged(field) => {
                self.name_field = field;
                Task::none()
            }
            Msg::MemoryFieldChanged(field) => {
                self.memory_field = field;
                Task::none()
            }
            Msg::ClickedCreateMemory => {
                self.status = Status::CreatingMemory;

                let new_memory = NewMemory {
                    memory_uuid: MemoryUuid::new(),
                    content: self.memory_field.clone(),
                    person_name: PersonName::from_string(self.name_field.clone()),
                };

                Task::perform(
                    async move { create_new_memory(&worker, new_memory).await },
                    Msg::CreatedMemory,
                )
            }
            Msg::CreatedMemory(result) => {
                self.status = match result {
                    Ok(_) => Status::Done,
                    Err(err) => Status::FailedCreatingMemory(err),
                };

                Task::none()
            }
        }
    }
}

fn status_view(status: &Status) -> Element<Msg> {
    match status {
        Status::Ready => w::text("Ready").into(),
        Status::CreatingMemory => w::text("Creating Memory...").into(),
        Status::Done => w::text("Memory created successfully!").into(),
        Status::FailedCreatingMemory(err) => w::text(format!("Error: {}", err)).into(),
    }
}

async fn create_new_memory(worker: &Worker, new_memory: NewMemory) -> Result<MemoryUuid, String> {
    worker.create_memory(new_memory).await
}
