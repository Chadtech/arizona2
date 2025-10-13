use crate::nice_display::{NiceDisplay, NiceError};

use super::job_uuid::JobUuid;

#[derive(Debug, Clone)]
pub struct Job {
    uuid: JobUuid,
    kind: JobKind,
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

    // Public getters to allow UI to render job information
    pub fn kind(&self) -> &JobKind {
        &self.kind
    }

    pub fn uuid(&self) -> &JobUuid {
        &self.uuid
    }

    pub fn to_info_string(&self) -> String {
        format!(
            "{}, uuid: {}",
            self.kind.to_name(),
            self.uuid.to_uuid().to_string()
        )
    }
}
