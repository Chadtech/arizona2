use super::s;
use crate::capability::job::JobCapability;
use crate::domain::job::JobKind;
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
    AddingPing,
    AddPingOk,
    AddPingErr(String),
}

#[derive(Debug, Clone)]
pub enum Msg {
    // Placeholder message variants for future interactions
    ClickedRefresh,
    Refreshed,
    ClickedAddPing,
    AddedPing(Result<(), String>),
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
        // Status message text
        let status_view: Element<Msg> = match &self.status {
            Status::Ready => w::text("Ready").into(),
            Status::AddingPing => w::text("Adding ping job...").into(),
            Status::AddPingOk => w::text("Ping job enqueued").into(),
            Status::AddPingErr(err) => w::text(format!("Failed to add ping job: {}", err)).into(),
        };

        // Disable the button while adding to prevent duplicates
        let add_button = match self.status {
            Status::AddingPing => w::button("Adding..."),
            _ => w::button("Add Ping Job").on_press(Msg::ClickedAddPing),
        };

        w::column![
            w::text("Jobs"),
            w::text("This is a placeholder for the Jobs page."),
            w::row![
                add_button,
                w::button("Refresh").on_press(Msg::ClickedRefresh),
            ]
            .spacing(s::S4),
            status_view
        ]
        .spacing(s::S4)
        .into()
    }

    pub fn update(&mut self, worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::ClickedRefresh => {
                // In the skeleton we just immediately yield a Refreshed message.
                Task::done(Msg::Refreshed)
            }
            Msg::Refreshed => {
                // No state changes yet; just a placeholder.
                Task::none()
            }
            Msg::ClickedAddPing => {
                self.status = Status::AddingPing;
                let worker = worker.clone();
                Task::perform(async move { worker.unshift_job(JobKind::Ping).await }, Msg::AddedPing)
            }
            Msg::AddedPing(res) => {
                match res {
                    Ok(()) => {
                        self.status = Status::AddPingOk;
                    }
                    Err(err) => {
                        self.status = Status::AddPingErr(err);
                    }
                }
                Task::none()
            }
        }
    }

    pub fn to_storage(&self) -> Storage {
        Storage {}
    }
}
