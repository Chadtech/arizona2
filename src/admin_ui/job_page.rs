use super::s;
use crate::capability::job::JobCapability;
use crate::domain::job::{Job, JobKind};
use crate::worker::Worker;
use iced::widget::container;
use iced::{widget as w, Color, Element, Length, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    add_ping_status: AddPingStatus,
    get_jobs_status: GetJobsStatus,
}

enum GetJobsStatus {
    Fetching,
    Error(String),
    GotJobs(Vec<Job>),
}

enum AddPingStatus {
    Ready,
    AddingPing,
    AddPingOk,
    AddPingErr(String),
}

#[derive(Debug, Clone)]
pub enum Msg {
    ClickedRefresh,
    ClickedAddPing,
    AddedPing(Result<(), String>),
    LoadedRecent(Result<Vec<Job>, String>),
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
        Self {
            add_ping_status: AddPingStatus::Ready,
            get_jobs_status: GetJobsStatus::Fetching,
        }
    }

    pub fn update(&mut self, worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::ClickedRefresh => {
                let worker = worker.clone();
                Task::perform(get_jobs(worker), |m| m)
            }
            Msg::ClickedAddPing => {
                self.add_ping_status = AddPingStatus::AddingPing;
                let worker = worker.clone();
                Task::perform(
                    async move { worker.unshift_job(JobKind::Ping).await },
                    Msg::AddedPing,
                )
            }
            Msg::AddedPing(res) => match res {
                Ok(()) => {
                    self.add_ping_status = AddPingStatus::AddPingOk;
                    let worker = worker.clone();
                    Task::perform(get_jobs(worker), |m| m)
                }
                Err(err) => {
                    self.add_ping_status = AddPingStatus::AddPingErr(err);
                    Task::none()
                }
            },
            Msg::LoadedRecent(res) => {
                self.get_jobs_status = match res {
                    Ok(names) => GetJobsStatus::GotJobs(names),
                    Err(err) => GetJobsStatus::Error(err),
                };
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<Msg> {
        // Status message text
        let status_view: Element<Msg> = match &self.add_ping_status {
            AddPingStatus::Ready => w::text("Ready").into(),
            AddPingStatus::AddingPing => w::text("Adding ping job...").into(),
            AddPingStatus::AddPingOk => w::text("Ping job enqueued").into(),
            AddPingStatus::AddPingErr(err) => {
                w::text(format!("Failed to add ping job: {}", err)).into()
            }
        };

        // Disable the button while adding to prevent duplicates
        let add_button = match self.add_ping_status {
            AddPingStatus::AddingPing => w::button("Adding..."),
            _ => w::button("Add Ping Job").on_press(Msg::ClickedAddPing),
        };

        let jobs_view: Element<Msg> = match &self.get_jobs_status {
            GetJobsStatus::Fetching => w::text("Loading jobs...").into(),
            GetJobsStatus::Error(err) => w::text(format!("Error loading jobs: {}", err)).into(),
            GetJobsStatus::GotJobs(jobs) => {
                if jobs.is_empty() {
                    w::text("No jobs yet").into()
                } else {
                    let mut col = w::column![];
                    for job in jobs {
                        col = col.push(w::text(job.to_info_string()));
                    }
                    w::scrollable(col).width(Length::Fill).into()
                }
            }
        };

        // Add a subtle border around the jobs list
        let jobs_container = w::container(jobs_view)
            .padding(s::S1)
            .width(Length::Fill)
            .style(|_| container::Style {
                border: iced::border::Border {
                    width: 1.0,
                    color: Color::from_rgb(0.8, 0.8, 0.8),
                    ..Default::default()
                },
                ..Default::default()
            });

        w::column![
            w::text("Jobs"),
            jobs_container,
            w::row![
                add_button,
                w::button("Refresh jobs list").on_press(Msg::ClickedRefresh),
            ]
            .spacing(s::S4),
            status_view
        ]
        .spacing(s::S4)
        .width(Length::Fill)
        .into()
    }

    pub fn to_storage(&self) -> Storage {
        Storage {}
    }
}

pub async fn get_jobs(worker: Arc<Worker>) -> Msg {
    let jobs_result = worker
        .recent_jobs(100)
        .await
        .map(|jobs| jobs.into_iter().collect::<Vec<_>>());

    Msg::LoadedRecent(jobs_result)
}
