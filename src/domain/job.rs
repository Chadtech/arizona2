use super::job_uuid::JobUuid;
use crate::nice_display::{NiceDisplay, NiceError};
use chrono::{DateTime, Utc};
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
}

pub enum ParseError {
    UnknownJobName(String),
}

impl NiceDisplay for ParseError {
    fn message(&self) -> String {
        match self {
            ParseError::UnknownJobName(name) => format!("Unknown job name: {}", name),
        }
    }
}

impl JobKind {
    pub fn to_name(&self) -> String {
        match self {
            JobKind::Ping => "ping".to_string(),
        }
    }
}

impl Display for JobUuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // write!(f, "{}", self.to_uuid().unwrap_or_default())
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
    ) -> Result<Job, ParseError> {
        let job_kid = JobKind::parse(name)?;

        Ok(Job {
            uuid,
            kind: job_kid,
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
    pub fn parse(uuid: JobUuid, name: String) -> Result<PoppedJob, ParseError> {
        let job_kid = JobKind::parse(name)?;

        Ok(PoppedJob {
            uuid,
            kind: job_kid,
        })
    }
}

impl JobKind {
    pub fn parse(name: String) -> Result<JobKind, ParseError> {
        match name.as_str() {
            "ping" => Ok(JobKind::Ping),
            _ => Err(ParseError::UnknownJobName(name)),
        }
    }
}
