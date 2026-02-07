use super::s;
use crate::capability::job::JobCapability;
use crate::domain::job::{Job, JobKind, JobStatus};
use crate::domain::job_uuid::JobUuid;
use crate::job_runner::{self, RunNextJobResult};
use crate::nice_display::NiceDisplay;
use crate::worker::Worker;
use iced::widget::container;
use iced::{clipboard, widget as w, Alignment, Background, Color, Element, Length, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    add_ping_status: AddPingStatus,
    get_jobs_status: GetJobsStatus,
    process_next_status: ProcessNextStatus,
    selected_job_status: SelectedJobStatus,
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
    Resetting,
    ResetOk,
    ResetErr(String),
}

enum SelectedJobStatus {
    None,
    Loading(JobUuid),
    Loaded(SelectedJobModel),
    Error(String),
}

struct SelectedJobModel {
    job: Job,
    delete_status: DeleteStatus,
    reset_status: ResetJobStatus,
}

enum DeleteStatus {
    Ready,
    Confirming,
    Deleting,
    Deleted,
    Error(String),
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
    ClickedSelectJob(JobUuid),
    LoadedJob(Result<Option<Job>, String>),
    ClickedDeleteSelected,
    ClickedConfirmDelete(JobUuid),
    ClickedCancelDelete,
    DeletedJob(Result<JobUuid, String>),
    ClickedCopyJobUuid(String),
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
            selected_job_status: SelectedJobStatus::None,
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
                if let SelectedJobStatus::Loaded(selected_job) = &mut self.selected_job_status {
                    if selected_job.job.uuid() == &job_uuid {
                        selected_job.reset_status = ResetJobStatus::Resetting;
                    }
                }
                let worker = worker.clone();
                Task::perform(reset_job(worker, job_uuid), Msg::ResetJobResult)
            }
            Msg::ResetJobResult(res) => match res {
                Ok(job_uuid) => {
                    if let SelectedJobStatus::Loaded(selected_job) = &mut self.selected_job_status {
                        if selected_job.job.uuid() == &job_uuid {
                            selected_job.reset_status = ResetJobStatus::ResetOk;
                        }
                    }
                    let worker = worker.clone();
                    Task::perform(get_jobs(worker), |m| m)
                }
                Err(err) => {
                    if let SelectedJobStatus::Loaded(selected_job) = &mut self.selected_job_status {
                        selected_job.reset_status = ResetJobStatus::ResetErr(err);
                    }
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
            Msg::ClickedSelectJob(job_uuid) => {
                self.selected_job_status = SelectedJobStatus::Loading(job_uuid.clone());
                let worker = worker.clone();
                Task::perform(get_job(worker, job_uuid), Msg::LoadedJob)
            }
            Msg::LoadedJob(res) => {
                self.selected_job_status = match res {
                    Ok(Some(job)) => SelectedJobStatus::Loaded(SelectedJobModel {
                        job,
                        delete_status: DeleteStatus::Ready,
                        reset_status: ResetJobStatus::Ready,
                    }),
                    Ok(None) => SelectedJobStatus::Error("Job not found".to_string()),
                    Err(err) => SelectedJobStatus::Error(err),
                };
                Task::none()
            }
            Msg::ClickedDeleteSelected => {
                if let SelectedJobStatus::Loaded(selected_job) = &mut self.selected_job_status {
                    selected_job.delete_status = DeleteStatus::Confirming;
                }
                Task::none()
            }
            Msg::ClickedConfirmDelete(job_uuid) => {
                if let SelectedJobStatus::Loaded(selected_job) = &mut self.selected_job_status {
                    selected_job.delete_status = DeleteStatus::Deleting;
                }
                let worker = worker.clone();
                Task::perform(delete_job(worker, job_uuid), Msg::DeletedJob)
            }
            Msg::ClickedCancelDelete => {
                if let SelectedJobStatus::Loaded(selected_job) = &mut self.selected_job_status {
                    selected_job.delete_status = DeleteStatus::Ready;
                }
                Task::none()
            }
            Msg::DeletedJob(res) => match res {
                Ok(_) => {
                    if let SelectedJobStatus::Loaded(selected_job) = &mut self.selected_job_status {
                        selected_job.delete_status = DeleteStatus::Deleted;
                    }
                    self.selected_job_status = SelectedJobStatus::None;
                    let worker = worker.clone();
                    Task::perform(get_jobs(worker), |m| m)
                }
                Err(err) => {
                    if let SelectedJobStatus::Loaded(selected_job) = &mut self.selected_job_status {
                        selected_job.delete_status = DeleteStatus::Error(err);
                    }
                    Task::none()
                }
            },
            Msg::ClickedCopyJobUuid(uuid) => clipboard::write(uuid),
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
                        let status_color = match job.status() {
                            JobStatus::Finished => s::GREEN_SOFT,
                            JobStatus::Failed => s::RED_SOFT,
                            JobStatus::NotStarted => s::GRAY_MID,
                        };

                        let job_label =
                            format!("{}, uuid: {}", job.kind_label(), job.uuid().to_string());

                        let row = w::row![
                            w::text(job_label),
                            w::text(job.status_label()).color(status_color),
                        ]
                        .spacing(s::S4)
                        .align_y(Alignment::Center)
                        .width(Length::Fill);

                        col = col.push(
                            w::button(row)
                                .padding([s::S1, s::S2])
                                .style(job_row_style)
                                .width(Length::Fill)
                                .on_press(Msg::ClickedSelectJob(job.uuid().clone())),
                        );
                    }
                    w::scrollable(col)
                        .width(Length::Fill)
                        .height(Length::Fixed(s::LIST_HEIGHT))
                        .into()
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
                    color: s::GRAY_SOFT,
                    ..Default::default()
                },
                ..Default::default()
            });

        w::column![
            w::text("Jobs"),
            jobs_container,
            selected_job_view(&self.selected_job_status),
            w::row![
                process_next_button,
                add_button,
                w::button("Refresh jobs list").on_press(Msg::ClickedRefresh),
            ]
            .spacing(s::S4),
            status_view,
            process_status_view,
        ]
        .spacing(s::S4)
        .width(Length::Fill)
        .into()
    }

    pub fn to_storage(&self) -> Storage {
        Storage {}
    }
}

fn selected_job_view(selected: &SelectedJobStatus) -> Element<'_, Msg> {
    let content: Element<Msg> = match selected {
        SelectedJobStatus::None => w::text("Select a job to see details").into(),
        SelectedJobStatus::Loading(_) => w::text("Loading job details...").into(),
        SelectedJobStatus::Error(err) => w::text(format!("Job load error: {}", err)).into(),
        SelectedJobStatus::Loaded(selected_job) => {
            let status_color = match selected_job.job.status() {
                JobStatus::Finished => s::GREEN_SOFT,
                JobStatus::Failed => s::RED_SOFT,
                JobStatus::NotStarted => s::GRAY_MID,
            };

            let started_at = format_job_time("Started", selected_job.job.started_at());
            let finished_at = format_job_time("Finished", selected_job.job.finished_at());
            let deleted_at = format_job_time("Deleted", selected_job.job.deleted_at());

            let error_text = match selected_job.job.error() {
                Some(err) => format!("Error: {}", err),
                None => "Error: none".to_string(),
            };

            let reset_controls: Element<Msg> = match &selected_job.reset_status {
                ResetJobStatus::Resetting => w::text("Resetting job...").into(),
                ResetJobStatus::ResetOk => w::text("Job reset").into(),
                ResetJobStatus::ResetErr(err) => w::text(format!("Reset failed: {}", err)).into(),
                ResetJobStatus::Ready => w::button("Reset job")
                    .on_press(Msg::ClickedResetJob(selected_job.job.uuid().clone()))
                    .into(),
            };

            let delete_controls: Element<Msg> = match &selected_job.delete_status {
                DeleteStatus::Confirming => w::row![
                    w::text("Delete this job permanently?"),
                    w::button("Confirm")
                        .on_press(Msg::ClickedConfirmDelete(selected_job.job.uuid().clone())),
                    w::button("Cancel").on_press(Msg::ClickedCancelDelete),
                ]
                .spacing(s::S4)
                .into(),
                DeleteStatus::Deleting => w::text("Deleting job...").into(),
                DeleteStatus::Deleted => w::text("Job deleted").into(),
                DeleteStatus::Error(err) => w::text(format!("Delete failed: {}", err)).into(),
                DeleteStatus::Ready => w::button("Delete job")
                    .on_press(Msg::ClickedDeleteSelected)
                    .into(),
            };

            let action_row: Element<Msg> = w::row![reset_controls, delete_controls]
                .spacing(s::S4)
                .into();

            w::column![
                w::text("Selected Job"),
                w::text(format!("Kind: {}", selected_job.job.kind_label())),
                w::row![
                    w::text(format!("UUID: {}", selected_job.job.uuid().to_string())),
                    w::button(w::text("Copy").size(s::S3))
                        .style(w::button::text)
                        .padding(0)
                        .on_press(Msg::ClickedCopyJobUuid(
                            selected_job.job.uuid().to_string()
                        )),
                ]
                .spacing(s::S2),
                w::row![
                    w::text("Status:"),
                    w::text(selected_job.job.status_label()).color(status_color)
                ]
                .spacing(s::S2),
                w::text(started_at),
                w::text(finished_at),
                w::text(deleted_at),
                w::text(error_text),
                action_row
            ]
            .spacing(s::S2)
            .into()
        }
    };

    w::container(content)
        .padding(s::S2)
        .width(Length::Fill)
        .style(|_| container::Style {
            border: iced::border::Border {
                width: 1.0,
                color: s::GRAY_DEEP,
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn job_row_style(_theme: &iced::Theme, status: w::button::Status) -> w::button::Style {
    let mut style = w::button::Style::default();
    style.text_color = s::GRAY_VERY_SOFT;

    match status {
        w::button::Status::Hovered | w::button::Status::Pressed => {
            style.background = Some(Background::Color(s::GRAY_VERY_DEEP));
        }
        _ => {}
    }

    style
}

fn format_job_time(label: &str, timestamp: Option<chrono::DateTime<chrono::Utc>>) -> String {
    match timestamp {
        Some(time) => format!("{}: {}", label, time.format("%Y-%m-%d %H:%M:%S UTC")),
        None => format!("{}: none", label),
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

async fn get_job(worker: Arc<Worker>, job_uuid: JobUuid) -> Result<Option<Job>, String> {
    worker
        .get_job_by_uuid(&job_uuid)
        .await
        .map_err(|err| format!("Error fetching job:\n{}", err))
}

async fn delete_job(worker: Arc<Worker>, job_uuid: JobUuid) -> Result<JobUuid, String> {
    worker
        .delete_job(&job_uuid)
        .await
        .map_err(|err| format!("Error deleting job:\n{}", err))?;
    Ok(job_uuid)
}
