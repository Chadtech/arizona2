use super::s;
use crate::worker::Worker;
use iced::{widget as w, Element, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Skeleton Jobs page. This mirrors the structure of other pages
// (Model, Msg, view, update) but intentionally keeps the content minimal.

pub struct Model {
    status: Status,
}

enum Status {
    Ready,
}

#[derive(Debug, Clone)]
pub enum Msg {
    // Placeholder message variants for future interactions
    ClickedRefresh,
    Refreshed,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Storage {}

impl Default for Storage {
    fn default() -> Self {
        Self {}
    }
}

impl Model {
    pub fn new(_storage: &Storage) -> Self {
        Self { status: Status::Ready }
    }

    pub fn view(&self) -> Element<Msg> {
        let status_view: Element<Msg> = match self.status {
            Status::Ready => w::text("Ready").into(),
        };

        w::column![
            w::text("Jobs"),
            w::text("This is a placeholder for the Jobs page."),
            w::button("Refresh").on_press(Msg::ClickedRefresh),
            status_view
        ]
        .spacing(s::S4)
        .into()
    }

    pub fn update(&mut self, _worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::ClickedRefresh => {
                // In the skeleton we just immediately yield a Refreshed message.
                Task::done(Msg::Refreshed)
            }
            Msg::Refreshed => {
                // No state changes yet; just a placeholder.
                Task::none()
            }
        }
    }

    pub fn to_storage(&self) -> Storage {
        Storage {}
    }
}
