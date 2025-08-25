use crate::admin_ui::s;
use crate::worker::Worker;
use iced::{widget as w, Element, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    name_field: String,
    memory_field: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    NameFieldChanged(String),
    MemoryFieldChanged(String),
    ClickedCreateMemory,
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
        }
    }

    pub fn view(&self) -> Element<Msg> {
        w::column![
            w::text("Person Name"),
            w::text_input("", &self.name_field).on_input(Msg::NameFieldChanged),
            w::text("Memory"),
            w::text_input("", &self.memory_field).on_input(Msg::MemoryFieldChanged),
            w::button("Create Memory").on_press(Msg::ClickedCreateMemory),
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
                todo!("Implement memory creation logic")
            }
        }
    }
}
