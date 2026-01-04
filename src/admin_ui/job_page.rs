use super::s;
use crate::capability::job::JobCapability;
use crate::domain::job::{Job, JobKind};
use crate::nice_display::NiceDisplay;
use crate::job_runner::{self, RunNextJobResult};
use crate::worker::Worker;
use iced::widget::container;
use iced::{widget as w, Color, Element, Length, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    add_ping_status: AddPingStatus,
    get_jobs_status: GetJobsStatus,
    process_next_status: ProcessNextStatus,
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

enum ProcessNextStatus {
    Ready,
    Processing,
    Processed,
    NoJob,
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum Msg {
    ClickedRefresh,
    ClickedAddPing,
    AddedPing(Result<(), String>),
    ClickedProcessNext,
    ProcessedNext(Result<RunNextJobResult, String>),
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
            process_next_status: ProcessNextStatus::Ready,
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
            Msg::ClickedProcessNext => {
                self.process_next_status = ProcessNextStatus::Processing;
                let worker = worker.clone();
                Task::perform(process_next_job(worker), Msg::ProcessedNext)
            }
            Msg::ProcessedNext(res) => match res {
                Ok(RunNextJobResult::RanJob) => {
                    self.process_next_status = ProcessNextStatus::Processed;
                    let worker = worker.clone();
                    Task::perform(get_jobs(worker), |m| m)
                }
                Ok(RunNextJobResult::NoJob) => {
                    self.process_next_status = ProcessNextStatus::NoJob;
                    Task::none()
                }
                Err(err) => {
                    self.process_next_status = ProcessNextStatus::Failed(err);
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

    pub fn view(&self) -> Element<'_, Msg> {
        // Status message text
        let status_view: Element<Msg> = match &self.add_ping_status {
            AddPingStatus::Ready => w::text("Ready").into(),
            AddPingStatus::AddingPing => w::text("Adding ping job...").into(),
            AddPingStatus::AddPingOk => w::text("Ping job enqueued").into(),
            AddPingStatus::AddPingErr(err) => {
                w::text(format!("Failed to add ping job: {}", err)).into()
            }
        };

        let process_status_view: Element<Msg> = match &self.process_next_status {
            ProcessNextStatus::Ready => w::text("Ready to process next job").into(),
            ProcessNextStatus::Processing => w::text("Processing next job...").into(),
            ProcessNextStatus::Processed => w::text("Processed next job").into(),
            ProcessNextStatus::NoJob => w::text("No jobs to process").into(),
            ProcessNextStatus::Failed(err) => {
                w::text(format!("Failed to process job: {}", err)).into()
            }
        };

        // Disable the button while adding to prevent duplicates
        let add_button = match self.add_ping_status {
            AddPingStatus::AddingPing => w::button("Adding..."),
            _ => w::button("Add Ping Job").on_press(Msg::ClickedAddPing),
        };

        let process_next_button = match self.process_next_status {
            ProcessNextStatus::Processing => w::button("Processing..."),
            _ => w::button("Process Next Job").on_press(Msg::ClickedProcessNext),
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
                process_next_button,
                add_button,
                w::button("Refresh jobs list").on_press(Msg::ClickedRefresh),
            ]
            .spacing(s::S4),
            status_view,
            process_status_view
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

async fn process_next_job(worker: Arc<Worker>) -> Result<RunNextJobResult, String> {
    let random_seed = worker.get_random_seed()?;
    job_runner::run_one_job(worker.as_ref().clone(), random_seed)
        .await
        .map_err(|err| err.to_nice_error().to_string())
}
