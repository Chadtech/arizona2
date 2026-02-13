pub mod person_waiting;
pub mod process_message;
pub mod send_message_to_scene;
pub mod person_action_handler;

use super::job_uuid::JobUuid;
use crate::domain::job::person_waiting::PersonWaitingJob;
use crate::domain::job::send_message_to_scene::SendMessageToSceneJob;
use crate::nice_display::NiceDisplay;
use chrono::{DateTime, Utc};
use process_message::ProcessMessageJob;
use serde_json;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct Job {
    uuid: JobUuid,
    kind: JobKind,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    error: Option<String>,
    deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct PoppedJob {
    pub uuid: JobUuid,
    pub kind: JobKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Finished,
    Failed,
    InProgress,
    NotStarted,
}

#[derive(Debug, Clone)]
pub enum JobKind {
    Ping,
    SendMessageToScene(SendMessageToSceneJob),
    ProcessMessage(ProcessMessageJob),
    PersonWaiting(PersonWaitingJob),
}

pub enum ParseError {
    UnknownJobName(String),
    NoJobDataForJobThatReuiresIt { job_name: String },
    FailedToParseJobData { job_name: String, details: String },
}

impl NiceDisplay for ParseError {
    fn message(&self) -> String {
        match self {
            ParseError::UnknownJobName(name) => format!("Unknown job name: {}", name),
            ParseError::NoJobDataForJobThatReuiresIt { job_name } => {
                format!(
                    "No job data provided for a \"{}\" job that requires it",
                    job_name
                )
            }
            ParseError::FailedToParseJobData { job_name, details } => {
                format!(
                    "Failed to parse job data for a \"{}\" job\n\n{}",
                    job_name, details
                )
            }
        }
    }
}

impl JobKind {
    pub fn to_name(&self) -> String {
        match self {
            JobKind::Ping => "ping".to_string(),
            JobKind::SendMessageToScene(_) => "send message to scene".to_string(),
            JobKind::ProcessMessage(_) => "process message".to_string(),
            JobKind::PersonWaiting(_) => "person waiting".to_string(),
        }
    }

    pub fn to_data(&self) -> Result<Option<serde_json::Value>, String> {
        match self {
            JobKind::Ping => Ok(None),
            JobKind::SendMessageToScene(job) => {
                let data = serde_json::to_value(job)
                    .map_err(|err| format!("Failed to serialize SendMessageToSceneJob: {}", err))?;
                Ok(Some(data))
            }
            JobKind::ProcessMessage(job) => {
                let data = serde_json::to_value(job)
                    .map_err(|err| format!("Failed to serialize ProcessMessageJob: {}", err))?;
                Ok(Some(data))
            }
            JobKind::PersonWaiting(job) => {
                let data = serde_json::to_value(job)
                    .map_err(|err| format!("Failed to serialize PersonWaitingJob: {}", err))?;
                Ok(Some(data))
            }
        }
    }
}

impl Display for JobUuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            JobUuid::Real(uuid) => uuid.to_string(),
            JobUuid::Test(id) => id.to_string(),
        };

        write!(f, "{}", str)
    }
}

impl Job {
    pub fn status(&self) -> JobStatus {
        if self.finished_at.is_some() {
            JobStatus::Finished
        } else if self.error.is_some() {
            JobStatus::Failed
        } else if self.started_at.is_some() {
            JobStatus::InProgress
        } else {
            JobStatus::NotStarted
        }
    }

    pub fn status_label(&self) -> String {
        match self.status() {
            JobStatus::Finished => {
                match self.finished_at {
                    Some(finished_at) => {
                        let date = finished_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
                        format!("finished at {}", date)
                    }
                    None => "finished".to_string(),
                }
            }
            JobStatus::Failed => "failed".to_string(),
            JobStatus::InProgress => "not finished".to_string(),
            JobStatus::NotStarted => "not started".to_string(),
        }
    }

    pub fn kind_label(&self) -> String {
        self.kind.to_name()
    }

    pub fn parse(
        uuid: JobUuid,
        started_at: Option<DateTime<Utc>>,
        finished_at: Option<DateTime<Utc>>,
        error: Option<String>,
        deleted_at: Option<DateTime<Utc>>,
        name: String,
        maybe_data: Option<serde_json::Value>,
    ) -> Result<Job, ParseError> {
        let job_kind = JobKind::parse(name, maybe_data)?;

        Ok(Job {
            uuid,
            kind: job_kind,
            started_at,
            finished_at,
            error,
            deleted_at,
        })
    }

    pub fn to_info_string(&self) -> String {
        format!(
            "{}, uuid: {}, {}",
            self.kind.to_name(),
            self.uuid.to_string(),
            self.status_label()
        )
    }

    pub fn uuid(&self) -> &JobUuid {
        &self.uuid
    }

    pub fn finished_at(&self) -> Option<DateTime<Utc>> {
        self.finished_at
    }

    pub fn started_at(&self) -> Option<DateTime<Utc>> {
        self.started_at
    }

    pub fn error(&self) -> Option<&String> {
        self.error.as_ref()
    }

    pub fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
}

impl PoppedJob {
    pub fn parse(
        uuid: JobUuid,
        name: String,
        maybe_data: Option<serde_json::Value>,
    ) -> Result<PoppedJob, ParseError> {
        let job_kid = JobKind::parse(name, maybe_data)?;

        Ok(PoppedJob {
            uuid,
            kind: job_kid,
        })
    }
}

impl JobKind {
    pub fn parse(
        name: String,
        maybe_data: Option<serde_json::Value>,
    ) -> Result<JobKind, ParseError> {
        match name.as_str() {
            "ping" => Ok(JobKind::Ping),
            "send message to scene" => match maybe_data {
                None => Err(ParseError::NoJobDataForJobThatReuiresIt { job_name: name }),
                Some(data) => {
                    let job: SendMessageToSceneJob =
                        serde_json::from_value(data).map_err(|error| {
                            ParseError::FailedToParseJobData {
                                job_name: name.clone(),
                                details: error.to_string(),
                            }
                        })?;

                    Ok(JobKind::SendMessageToScene(job))
                }
            },
            "process message" => match maybe_data {
                None => Err(ParseError::NoJobDataForJobThatReuiresIt { job_name: name }),
                Some(data) => {
                    let job: ProcessMessageJob = serde_json::from_value(data).map_err(|error| {
                        ParseError::FailedToParseJobData {
                            job_name: name.clone(),
                            details: error.to_string(),
                        }
                    })?;

                    Ok(JobKind::ProcessMessage(job))
                }
            },
            "person waiting" => match maybe_data {
                None => Err(ParseError::NoJobDataForJobThatReuiresIt { job_name: name }),
                Some(data) => {
                    let job: PersonWaitingJob = serde_json::from_value(data).map_err(|error| {
                        ParseError::FailedToParseJobData {
                            job_name: name.clone(),
                            details: error.to_string(),
                        }
                    })?;

                    Ok(JobKind::PersonWaiting(job))
                }
            },
            _ => Err(ParseError::UnknownJobName(name)),
        }
    }
}
