use crate::nice_display::NiceDisplay;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersonWaitingJob {
    until: DateTime<Utc>,
}

pub enum Error {}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            _ => "An error occurred".to_string(),
        }
    }
}

impl PersonWaitingJob {
    pub fn new(duration: i64) -> Self {
        let until = Utc::now() + Duration::seconds(duration.max(0));
        Self { until }
    }
    pub async fn run<W>(&self, worker: &W) -> Result<(), Error> {
        // TODO
        Ok(())
    }
}
