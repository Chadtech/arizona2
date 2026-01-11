use super::s;
use crate::capability::job::JobCapability;
use crate::domain::job::{Job, JobKind};
use crate::domain::job_uuid::JobUuid;
use crate::job_runner::{self, RunNextJobResult};
use crate::nice_display::NiceDisplay;
use crate::worker::Worker;
use iced::widget::container;
use iced::{widget as w, Alignment, Color, Element, Length, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    add_ping_status: AddPingStatus,
    get_jobs_status: GetJobsStatus,
    process_next_status: ProcessNextStatus,
    reset_job_status: ResetJobStatus,
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
    Processed { job_uuid: String, job_kind: String },
    Deferred { job_uuid: String, job_kind: String },
    NoJob,
    Failed(String),
}

enum ResetJobStatus {
    Ready,
    Resetting(JobUuid),
    ResetOk(JobUuid),
    ResetErr(String),
}

#[derive(Debug, Clone)]
pub enum Msg {
    ClickedRefresh,
    ClickedAddPing,
    AddedPing(Result<(), String>),
    ClickedProcessNext,
    ProcessedNext(Result<RunNextJobResult, String>),
    ClickedResetJob(JobUuid),
    ResetJobResult(Result<JobUuid, String>),
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
            reset_job_status: ResetJobStatus::Ready,
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
                Ok(RunNextJobResult::RanJob { job_uuid, job_kind }) => {
                    self.process_next_status = ProcessNextStatus::Processed {
                        job_uuid: job_uuid.to_string(),
                        job_kind,
                    };
                    let worker = worker.clone();
                    Task::perform(get_jobs(worker), |m| m)
                }
                Ok(RunNextJobResult::Deferred { job_uuid, job_kind }) => {
                    self.process_next_status = ProcessNextStatus::Deferred {
                        job_uuid: job_uuid.to_string(),
                        job_kind,
                    };
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
            Msg::ClickedResetJob(job_uuid) => {
                self.reset_job_status = ResetJobStatus::Resetting(job_uuid.clone());
                let worker = worker.clone();
                Task::perform(reset_job(worker, job_uuid), Msg::ResetJobResult)
            }
            Msg::ResetJobResult(res) => match res {
                Ok(job_uuid) => {
                    self.reset_job_status = ResetJobStatus::ResetOk(job_uuid);
                    let worker = worker.clone();
                    Task::perform(get_jobs(worker), |m| m)
                }
                Err(err) => {
                    self.reset_job_status = ResetJobStatus::ResetErr(err);
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
            ProcessNextStatus::Processed { job_uuid, job_kind } => {
                w::text(format!("Processed {} ({})", job_kind, job_uuid)).into()
            }
            ProcessNextStatus::Deferred { job_uuid, job_kind } => {
                w::text(format!("Deferred {} ({})", job_kind, job_uuid)).into()
            }
            ProcessNextStatus::NoJob => w::text("No jobs to process").into(),
            ProcessNextStatus::Failed(err) => {
                w::text(format!("Failed to process job: {}", err)).into()
            }
        };

        let reset_status_view: Element<Msg> = match &self.reset_job_status {
            ResetJobStatus::Ready => w::text("Ready to reset job").into(),
            ResetJobStatus::Resetting(job_uuid) => {
                w::text(format!("Resetting job {}", job_uuid)).into()
            }
            ResetJobStatus::ResetOk(job_uuid) => w::text(format!("Reset job {}", job_uuid)).into(),
            ResetJobStatus::ResetErr(err) => w::text(format!("Reset failed: {}", err)).into(),
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
            GetJobsStatus::Error(err) => w::text(format!("Error loading jobs:\n{}", err)).into(),
            GetJobsStatus::GotJobs(jobs) => {
                if jobs.is_empty() {
                    w::text("No jobs yet").into()
                } else {
                    let mut col = w::column![];
                    for job in jobs {
                        let reset_control: Element<Msg> = w::button("Reset")
                            .style(w::button::text)
                            .padding(s::S1)
                            .on_press(Msg::ClickedResetJob(job.uuid().clone()))
                            .into();

                        col = col.push(
                            w::row![w::text(job.to_info_string()), reset_control]
                                .spacing(s::S4)
                                .align_y(Alignment::Center),
                        );
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
            process_status_view,
            reset_status_view
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

async fn reset_job(worker: Arc<Worker>, job_uuid: JobUuid) -> Result<JobUuid, String> {
    worker
        .reset_job(&job_uuid)
        .await
        .map_err(|err| format!("Error resetting job:\n{}", err))?;
    Ok(job_uuid)
}
