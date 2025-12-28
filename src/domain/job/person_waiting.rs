use crate::nice_display::NiceDisplay;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersonWaitingJob {}

pub enum Error {}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            _ => "An error occurred".to_string(),
        }
    }
}

impl PersonWaitingJob {
    pub async fn run<W>(&self, worker: &W) -> Result<(), Error> {
        // TODO
        Ok(())
    }
}
