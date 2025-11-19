pub mod process_message;
pub mod send_message_to_scene;

use super::job_uuid::JobUuid;
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
    finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct PoppedJob {
    pub uuid: JobUuid,
    pub kind: JobKind,
}

#[derive(Debug, Clone)]
pub enum JobKind {
    Ping,
    SendMessageToScene(SendMessageToSceneJob),
    ProcessMessage(ProcessMessageJob),
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
    pub fn parse(
        uuid: JobUuid,
        finished_at: Option<DateTime<Utc>>,
        name: String,
        maybe_data: Option<serde_json::Value>,
    ) -> Result<Job, ParseError> {
        let job_kind = JobKind::parse(name, maybe_data)?;

        Ok(Job {
            uuid,
            kind: job_kind,
            finished_at,
        })
    }

    // Public getters to allow UI to render job information
    pub fn kind(&self) -> &JobKind {
        &self.kind
    }

    pub fn uuid(&self) -> &JobUuid {
        &self.uuid
    }

    pub fn to_info_string(&self) -> String {
        let status_string = match self.finished_at {
            Some(ts) => {
                let date: String = ts.format("%Y-%m-%d %H:%M:%S UTC").to_string();
                format!("finished at {}", date)
            }
            None => "not started".to_string(),
        };

        format!(
            "{}, uuid: {}, {}",
            self.kind.to_name(),
            self.uuid.to_string(),
            status_string
        )
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
            "send message" => match maybe_data {
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
            _ => Err(ParseError::UnknownJobName(name)),
        }
    }
}
