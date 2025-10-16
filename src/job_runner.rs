use crate::capability::job::JobCapability;
use crate::domain::job::{JobKind, PoppedJob};
use crate::nice_display::NiceDisplay;
use crate::worker;
use crate::worker::Worker;

pub enum Error {
    WorkerInitError(worker::InitError),
    RunJobError(RunJobError),
}

pub enum RunJobError {
    FailedToPopJob(String),
    FailedToMarkJobFinished(String),
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::WorkerInitError(err) => {
                format!("Worker initialization error\n{}", err.message())
            }
            Error::RunJobError(err) => err.message(),
        }
    }
}

impl NiceDisplay for RunJobError {
    fn message(&self) -> String {
        match self {
            RunJobError::FailedToPopJob(err) => {
                format!("Failed to pop next job\n{}", err)
            }
            RunJobError::FailedToMarkJobFinished(err) => {
                format!(
                    "I ran into the following problem trying to mark the job as finished\n{}",
                    err
                )
            }
        }
    }
}
pub async fn run() -> Result<(), Error> {
    let worker = Worker::new().await.map_err(Error::WorkerInitError)?;
    loop {
        run_next_job(worker.clone())
            .await
            .map_err(Error::RunJobError)?;
    }
}

async fn run_next_job<W: JobCapability>(worker: W) -> Result<(), RunJobError> {
    let job = match worker
        .pop_next_job()
        .await
        .map_err(RunJobError::FailedToPopJob)?
    {
        Some(j) => j,
        None => {
            return Ok(());
        }
    };


    match job.kind {
        JobKind::Ping => {
            println!("Pong");
        }
    }

    worker
        .mark_job_finished(&job.uuid)
        .await
        .map_err(RunJobError::FailedToMarkJobFinished)?;

    Ok(())
}
