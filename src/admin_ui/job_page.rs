use super::s;
use crate::capability::job::JobCapability;
use crate::capability::message::MessageCapability;
use crate::capability::reaction::ReactionPromptPreview;
use crate::domain::job::process_reaction_common::{self, SceneReactionTrigger};
use crate::domain::job::{Job, JobKind, JobStatus};
use crate::domain::job_uuid::JobUuid;
use crate::job_runner::{self, RunNextJobResult};
use crate::nice_display::NiceDisplay;
use crate::worker::Worker;
use iced::widget::container;
use iced::widget::scrollable;
use iced::{
    clipboard, time, widget as w, Alignment, Background, Element, Length, Subscription, Task,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const JOB_PAGE_SIZE: usize = 100;
const LOAD_MORE_THRESHOLD: f32 = 0.95;

pub struct Model {
    add_ping_status: AddPingStatus,
    get_jobs_status: GetJobsStatus,
    process_next_status: ProcessNextStatus,
    selected_job_status: SelectedJobStatus,
    auto_refresh: bool,
    jobs_limit: usize,
    jobs_scrollable_id: scrollable::Id,
    load_more_status: LoadMoreStatus,
    reset_failed_status: ResetFailedStatus,
}

enum GetJobsStatus {
    Fetching,
    Error(String),
    GotJobs(Vec<Job>),
}

enum LoadMoreStatus {
    Ready { can_load_more: bool },
    Loading,
    Exhausted,
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

enum ResetFailedStatus {
    Ready,
    Resetting,
    ResetOk,
    ResetErr(String),
}

enum SelectedJobStatus {
    None,
    Loading,
    Loaded(Box<SelectedJobModel>),
    Error(String),
}

struct SelectedJobModel {
    job: Job,
    delete_status: DeleteStatus,
    reset_status: ResetJobStatus,
    preview_status: PromptPreviewStatus,
}

enum PromptPreviewStatus {
    Ready,
    Loading,
    Loaded(ReactionPromptPreview),
    Error(String),
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
    ClickedResetAllFailedJobs,
    ResetAllFailedJobs(Result<(), String>),
    LoadedRecent(Result<Vec<Job>, String>),
    ClickedSelectJob(JobUuid),
    LoadedJob(Result<Option<Job>, String>),
    ClickedDeleteSelected,
    ClickedConfirmDelete(JobUuid),
    ClickedCancelDelete,
    DeletedJob(Result<JobUuid, String>),
    ClickedCopyJobUuid(String),
    ClickedCopyPromptPreview(String),
    ClickedRefreshSelected,
    ClickedPreviewSelectedJob,
    LoadedPromptPreview(Result<ReactionPromptPreview, String>),
    ClickedToggleAutoRefresh,
    AutoRefreshTick,
    JobListScrolled(scrollable::Viewport),
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Storage {}

impl Model {
    pub fn new(_storage: &Storage) -> Self {
        Self {
            add_ping_status: AddPingStatus::Ready,
            get_jobs_status: GetJobsStatus::Fetching,
            process_next_status: ProcessNextStatus::Ready,
            selected_job_status: SelectedJobStatus::None,
            auto_refresh: false,
            jobs_limit: JOB_PAGE_SIZE,
            jobs_scrollable_id: scrollable::Id::unique(),
            load_more_status: LoadMoreStatus::Ready {
                can_load_more: true,
            },
            reset_failed_status: ResetFailedStatus::Ready,
        }
    }

    pub fn update(&mut self, worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::ClickedRefresh => {
                let worker = worker.clone();
                Task::perform(get_jobs(worker, self.jobs_limit), |m| m)
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
                    Task::perform(get_jobs(worker, self.jobs_limit), |m| m)
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
                    Task::perform(get_jobs(worker, self.jobs_limit), |m| m)
                }
                Ok(RunNextJobResult::Deferred { job_uuid, job_kind }) => {
                    self.process_next_status = ProcessNextStatus::Deferred {
                        job_uuid: job_uuid.to_string(),
                        job_kind,
                    };
                    let worker = worker.clone();
                    Task::perform(get_jobs(worker, self.jobs_limit), |m| m)
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
                    let mut tasks = vec![];
                    let worker = worker.clone();
                    tasks.push(Task::perform(
                        get_jobs(worker.clone(), self.jobs_limit),
                        |m| m,
                    ));

                    if let SelectedJobStatus::Loaded(selected_job) = &self.selected_job_status {
                        if selected_job.job.uuid() == &job_uuid {
                            let worker = worker.clone();
                            tasks.push(Task::perform(get_job(worker, job_uuid), Msg::LoadedJob));
                        }
                    }

                    Task::batch(tasks)
                }
                Err(err) => {
                    if let SelectedJobStatus::Loaded(selected_job) = &mut self.selected_job_status {
                        selected_job.reset_status = ResetJobStatus::ResetErr(err);
                    }
                    Task::none()
                }
            },
            Msg::ClickedResetAllFailedJobs => {
                self.reset_failed_status = ResetFailedStatus::Resetting;
                let worker = worker.clone();
                Task::perform(reset_all_failed_jobs(worker), Msg::ResetAllFailedJobs)
            }
            Msg::ResetAllFailedJobs(res) => match res {
                Ok(()) => {
                    self.reset_failed_status = ResetFailedStatus::ResetOk;
                    let mut tasks = vec![];
                    let worker = worker.clone();
                    tasks.push(Task::perform(
                        get_jobs(worker.clone(), self.jobs_limit),
                        |m| m,
                    ));

                    if let SelectedJobStatus::Loaded(selected_job) = &self.selected_job_status {
                        let worker = worker.clone();
                        tasks.push(Task::perform(
                            get_job(worker, selected_job.job.uuid().clone()),
                            Msg::LoadedJob,
                        ));
                    }

                    Task::batch(tasks)
                }
                Err(err) => {
                    self.reset_failed_status = ResetFailedStatus::ResetErr(err);
                    Task::none()
                }
            },
            Msg::LoadedRecent(res) => {
                self.get_jobs_status = match res {
                    Ok(names) => GetJobsStatus::GotJobs(names),
                    Err(err) => GetJobsStatus::Error(err),
                };
                self.load_more_status = match &self.get_jobs_status {
                    GetJobsStatus::GotJobs(jobs) => {
                        if jobs.len() >= self.jobs_limit {
                            LoadMoreStatus::Ready {
                                can_load_more: true,
                            }
                        } else {
                            LoadMoreStatus::Exhausted
                        }
                    }
                    _ => LoadMoreStatus::Exhausted,
                };
                Task::none()
            }
            Msg::ClickedSelectJob(job_uuid) => {
                self.selected_job_status = SelectedJobStatus::Loading;
                let worker = worker.clone();
                Task::perform(get_job(worker, job_uuid), Msg::LoadedJob)
            }
            Msg::LoadedJob(res) => {
                self.selected_job_status = match res {
                    Ok(Some(job)) => SelectedJobStatus::Loaded(Box::new(SelectedJobModel {
                        job,
                        delete_status: DeleteStatus::Ready,
                        reset_status: ResetJobStatus::Ready,
                        preview_status: PromptPreviewStatus::Ready,
                    })),
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
            Msg::ClickedRefreshSelected => {
                if let SelectedJobStatus::Loaded(selected_job) = &self.selected_job_status {
                    let selected_job_uuid = selected_job.job.uuid().clone();

                    self.selected_job_status = SelectedJobStatus::Loading;

                    let worker = worker.clone();
                    return Task::perform(get_job(worker, selected_job_uuid), Msg::LoadedJob);
                }
                Task::none()
            }
            Msg::ClickedPreviewSelectedJob => {
                if let SelectedJobStatus::Loaded(selected_job) = &mut self.selected_job_status {
                    selected_job.preview_status = PromptPreviewStatus::Loading;
                    let worker = worker.clone();
                    let job = selected_job.job.clone();
                    return Task::perform(
                        preview_job_prompts(worker, job),
                        Msg::LoadedPromptPreview,
                    );
                }
                Task::none()
            }
            Msg::LoadedPromptPreview(result) => {
                if let SelectedJobStatus::Loaded(selected_job) = &mut self.selected_job_status {
                    selected_job.preview_status = match result {
                        Ok(preview) => PromptPreviewStatus::Loaded(preview),
                        Err(err) => PromptPreviewStatus::Error(err),
                    };
                }
                Task::none()
            }
            Msg::ClickedToggleAutoRefresh => {
                self.auto_refresh = !self.auto_refresh;
                Task::none()
            }
            Msg::AutoRefreshTick => {
                if self.auto_refresh {
                    let worker = worker.clone();
                    Task::perform(get_jobs(worker, self.jobs_limit), |m| m)
                } else {
                    Task::none()
                }
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
                    Task::perform(get_jobs(worker, self.jobs_limit), |m| m)
                }
                Err(err) => {
                    if let SelectedJobStatus::Loaded(selected_job) = &mut self.selected_job_status {
                        selected_job.delete_status = DeleteStatus::Error(err);
                    }
                    Task::none()
                }
            },
            Msg::ClickedCopyJobUuid(uuid) => clipboard::write(uuid),
            Msg::ClickedCopyPromptPreview(contents) => clipboard::write(contents),
            Msg::JobListScrolled(viewport) => {
                if self.should_load_more_jobs(viewport) {
                    self.load_more_status = LoadMoreStatus::Loading;
                    self.jobs_limit = self.jobs_limit.saturating_add(JOB_PAGE_SIZE);
                    let worker = worker.clone();
                    Task::perform(get_jobs(worker, self.jobs_limit), |m| m)
                } else {
                    Task::none()
                }
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

        let reset_failed_view: Element<Msg> = match &self.reset_failed_status {
            ResetFailedStatus::Ready => w::text("").into(),
            ResetFailedStatus::Resetting => w::text("Resetting failed jobs...").into(),
            ResetFailedStatus::ResetOk => w::text("Reset failed jobs").into(),
            ResetFailedStatus::ResetErr(err) => {
                w::text(format!("Failed to reset failed jobs: {}", err)).into()
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

        let reset_failed_button = match self.reset_failed_status {
            ResetFailedStatus::Resetting => w::button("Resetting failed jobs..."),
            _ => w::button("Reset failed jobs").on_press(Msg::ClickedResetAllFailedJobs),
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
                            JobStatus::InProgress => s::GOLD_SOFT,
                            JobStatus::NotStarted => s::GRAY_MID,
                        };

                        let job_label = format!("{}, uuid: {}", job.kind_label(), job.uuid());

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
                        .id(self.jobs_scrollable_id.clone())
                        .on_scroll(Msg::JobListScrolled)
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

        let auto_refresh_label = if self.auto_refresh {
            "Auto refresh: On"
        } else {
            "Auto refresh: Off"
        };

        w::column![
            w::text("Jobs"),
            jobs_container,
            selected_job_view(&self.selected_job_status),
            w::row![
                process_next_button,
                add_button,
                reset_failed_button,
                w::button("Refresh jobs list").on_press(Msg::ClickedRefresh),
                w::button(auto_refresh_label).on_press(Msg::ClickedToggleAutoRefresh),
            ]
            .spacing(s::S4),
            status_view,
            process_status_view,
            reset_failed_view,
        ]
        .spacing(s::S4)
        .width(Length::Fill)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Msg> {
        if self.auto_refresh {
            time::every(std::time::Duration::from_secs(2)).map(|_| Msg::AutoRefreshTick)
        } else {
            Subscription::none()
        }
    }

    pub fn to_storage(&self) -> Storage {
        Storage {}
    }

    fn should_load_more_jobs(&mut self, viewport: scrollable::Viewport) -> bool {
        let relative_offset = viewport.relative_offset();
        match &mut self.load_more_status {
            LoadMoreStatus::Ready { can_load_more } => {
                if relative_offset.y < LOAD_MORE_THRESHOLD {
                    *can_load_more = true;
                    return false;
                }

                if !*can_load_more {
                    return false;
                }

                *can_load_more = false;
                true
            }
            LoadMoreStatus::Loading => false,
            LoadMoreStatus::Exhausted => false,
        }
    }
}

pub fn initial_jobs_limit() -> usize {
    JOB_PAGE_SIZE
}

fn selected_job_view(selected: &SelectedJobStatus) -> Element<'_, Msg> {
    let content: Element<Msg> = match selected {
        SelectedJobStatus::None => w::text("Select a job to see details").into(),
        SelectedJobStatus::Loading => w::text("Loading job details...").into(),
        SelectedJobStatus::Error(err) => w::text(format!("Job load error: {}", err)).into(),
        SelectedJobStatus::Loaded(selected_job) => {
            let status_color = match selected_job.job.status() {
                JobStatus::Finished => s::GREEN_SOFT,
                JobStatus::Failed => s::RED_SOFT,
                JobStatus::InProgress => s::GOLD_SOFT,
                JobStatus::NotStarted => s::GRAY_MID,
            };

            let started_at = format_job_time("Started", selected_job.job.started_at());
            let finished_at = format_job_time("Finished", selected_job.job.finished_at());
            let deleted_at = format_job_time("Deleted", selected_job.job.deleted_at());

            let error_text = match selected_job.job.error() {
                Some(err) => format!("Error: {}", err),
                None => "Error: none".to_string(),
            };

            let data_text = match selected_job.job.data() {
                Ok(Some(data)) => match serde_json::to_string_pretty(&data) {
                    Ok(pretty) => format!("Data:\n{}", pretty),
                    Err(err) => format!("Data: <failed to format JSON: {}>", err),
                },
                Ok(None) => "Data: none".to_string(),
                Err(err) => format!("Data error: {}", err),
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

            let preview_controls: Element<Msg> = match &selected_job.preview_status {
                PromptPreviewStatus::Ready => w::button("Preview prompts")
                    .on_press(Msg::ClickedPreviewSelectedJob)
                    .into(),
                PromptPreviewStatus::Loading => w::text("Building prompt preview...").into(),
                PromptPreviewStatus::Loaded(preview) => {
                    let copy_text = format_prompt_preview(preview);
                    w::column![
                        w::row![
                            w::button("Preview prompts").on_press(Msg::ClickedPreviewSelectedJob),
                            w::button(w::text("Copy preview").size(s::S3))
                                .style(w::button::text)
                                .padding(s::S1)
                                .on_press(Msg::ClickedCopyPromptPreview(copy_text)),
                        ]
                        .spacing(s::S2),
                        prompt_preview_section(
                            "Thinking System Prompt",
                            &preview.thinking_system_prompt,
                        ),
                        prompt_preview_section(
                            "Thinking User Prompt",
                            &preview.thinking_user_prompt,
                        ),
                        prompt_preview_section(
                            "Action System Prompt",
                            &preview.action_system_prompt,
                        ),
                        prompt_preview_section("Action User Prompt", &preview.action_user_prompt),
                    ]
                    .spacing(s::S2)
                    .into()
                }
                PromptPreviewStatus::Error(err) => w::column![
                    w::button("Preview prompts").on_press(Msg::ClickedPreviewSelectedJob),
                    w::text(format!("Preview error: {}", err)),
                ]
                .spacing(s::S1)
                .into(),
            };

            w::column![
                w::row![
                    w::text("Selected Job"),
                    w::button("Refresh")
                        .style(w::button::text)
                        .padding(s::S1)
                        .on_press(Msg::ClickedRefreshSelected),
                ]
                .spacing(s::S2),
                w::text(format!("Kind: {}", selected_job.job.kind_label())),
                w::row![
                    w::text(format!("UUID: {}", selected_job.job.uuid())),
                    w::button(w::text("Copy").size(s::S3))
                        .style(w::button::text)
                        .padding(0)
                        .on_press(Msg::ClickedCopyJobUuid(selected_job.job.uuid().to_string())),
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
                w::text(data_text),
                action_row,
                preview_controls
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
    let mut style = w::button::Style {
        text_color: s::GRAY_VERY_SOFT,
        ..Default::default()
    };

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

fn prompt_preview_section<'a>(title: &'a str, body: &'a str) -> Element<'a, Msg> {
    w::container(
        w::column![
            w::text(title).size(s::S4).color(s::GRAY_VERY_SOFT),
            w::text(body),
        ]
        .spacing(s::S2),
    )
    .padding(s::S2)
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

fn format_prompt_preview(preview: &ReactionPromptPreview) -> String {
    format!(
        "Thinking System Prompt\n\n{}\n\nThinking User Prompt\n\n{}\n\nAction System Prompt\n\n{}\n\nAction User Prompt\n\n{}",
        preview.thinking_system_prompt,
        preview.thinking_user_prompt,
        preview.action_system_prompt,
        preview.action_user_prompt
    )
}

pub async fn get_jobs(worker: Arc<Worker>, limit: usize) -> Msg {
    let jobs_result = worker
        .recent_jobs(limit as i64)
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

async fn reset_all_failed_jobs(worker: Arc<Worker>) -> Result<(), String> {
    worker
        .reset_all_failed_jobs()
        .await
        .map_err(|err| format!("Error resetting failed jobs:\n{}", err))
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

async fn preview_job_prompts(
    worker: Arc<Worker>,
    job: Job,
) -> Result<ReactionPromptPreview, String> {
    match job.kind() {
        JobKind::ProcessMessage(process_message_job) => {
            let maybe_message = worker
                .get_message_by_uuid(&process_message_job.message_uuid)
                .await
                .map_err(|err| format!("Failed to load message for process message job: {}", err))?;
            let message = maybe_message.ok_or_else(|| "Message not found for process message job".to_string())?;

            process_reaction_common::preview_scene_reaction_prompts(
                worker.as_ref(),
                &process_message_job.recipient_person_uuid,
                &message.scene_uuid,
                SceneReactionTrigger::NewMessages,
            )
            .await
            .map_err(|err| err.message())
        }
        JobKind::ProcessPersonJoin(process_person_join_job) => {
            process_reaction_common::preview_scene_reaction_prompts(
                worker.as_ref(),
                &process_person_join_job.recipient_person_uuid,
                &process_person_join_job.scene_uuid,
                SceneReactionTrigger::PersonJoined {
                    joined_person_uuid: process_person_join_job.joined_person_uuid.clone(),
                },
            )
            .await
            .map_err(|err| err.message())
        }
        _ => Err("Prompt preview is currently supported only for process message and process person join jobs.".to_string()),
    }
}
