use crate::capability::person::PersonCapability;
use crate::domain::person_uuid::PersonUuid;
use crate::nice_display::NiceDisplay;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersonHibernatingJob {
    person_uuid: PersonUuid,
    started_at: DateTime<Utc>,
    duration_ms: i64,
    start_active_ms: i64,
}

pub enum Error {
    FailedToGetHibernationState(String),
    FailedToSetHibernationState(String),
}

pub enum HibernateOutcome {
    Ready,
    NotReady,
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::FailedToGetHibernationState(err) => {
                format!("Failed to get hibernation state: {}", err)
            }
            Error::FailedToSetHibernationState(err) => {
                format!("Failed to set hibernation state: {}", err)
            }
        }
    }
}

impl PersonHibernatingJob {
    pub fn new(person_uuid: PersonUuid, duration_ms: i64, start_active_ms: i64) -> Self {
        Self {
            person_uuid,
            started_at: Utc::now(),
            duration_ms: duration_ms.max(0),
            start_active_ms: start_active_ms.max(0),
        }
    }

    pub fn run_at_active_ms(&self) -> i64 {
        self.start_active_ms.saturating_add(self.duration_ms.max(0))
    }

    pub async fn run<W: PersonCapability>(
        &self,
        worker: &W,
        current_active_ms: i64,
    ) -> Result<HibernateOutcome, Error> {
        let person_uuid = self.person_uuid.clone();
        let _started_at = self.started_at;

        let is_hibernating = worker
            .is_person_hibernating(&person_uuid)
            .await
            .map_err(Error::FailedToGetHibernationState)?;
        if !is_hibernating {
            return Ok(HibernateOutcome::Ready);
        }

        let elapsed = current_active_ms.saturating_sub(self.start_active_ms);
        if elapsed < self.duration_ms {
            return Ok(HibernateOutcome::NotReady);
        }

        worker
            .set_person_hibernating(&person_uuid, false)
            .await
            .map_err(Error::FailedToSetHibernationState)?;

        Ok(HibernateOutcome::Ready)
    }
}
