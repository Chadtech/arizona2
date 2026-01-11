use crate::nice_display::NiceDisplay;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersonWaitingJob {
    #[serde(default)]
    duration_ms: i64,
    #[serde(default)]
    start_active_ms: i64,
}

pub enum Error {}

pub enum WaitOutcome {
    Ready,
    NotReady,
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            _ => "An error occurred".to_string(),
        }
    }
}

impl PersonWaitingJob {
    pub fn new(duration_ms: i64, start_active_ms: i64) -> Self {
        Self {
            duration_ms: duration_ms.max(0),
            start_active_ms: start_active_ms.max(0),
        }
    }

    pub fn run_at_active_ms(&self) -> i64 {
        self.start_active_ms
            .saturating_add(self.duration_ms.max(0))
    }

    pub fn run(&self, current_active_ms: i64) -> Result<WaitOutcome, Error> {
        let elapsed = current_active_ms.saturating_sub(self.start_active_ms);
        if elapsed >= self.duration_ms {
            Ok(WaitOutcome::Ready)
        } else {
            Ok(WaitOutcome::NotReady)
        }
    }
}
