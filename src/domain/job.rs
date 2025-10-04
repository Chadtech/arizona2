use crate::nice_display::{NiceDisplay, NiceError};

use super::job_uuid::JobUuid;

pub struct Job {
    uuid: JobUuid,
    kind: JobKind,
}

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

impl Job {
    pub fn parse(uuid: JobUuid, name: String) -> Result<Job, ParseError> {
        match name.as_str() {
            "ping" => Ok(Job {
                uuid,
                kind: JobKind::Ping,
            }),
            _ => Err(ParseError::UnknownJobName(name)),
        }
    }
}
