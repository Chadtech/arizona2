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
        JobKind::SendMessageToScene(job_data) => {
            // TODO implement sending message to scene
        }
    }

    worker
        .mark_job_finished(&job.uuid)
        .await
        .map_err(RunJobError::FailedToMarkJobFinished)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::job::JobCapability;
    use crate::domain::job::{JobKind, PoppedJob};
    use crate::domain::job_uuid::JobUuid;
    use std::collections::HashSet;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Clone, Default)]
    struct MockWorker {
        state: Arc<Mutex<MockState>>,
    }

    #[derive(Default)]
    struct MockState {
        jobs: Vec<PoppedJob>,
        finished_jobs: HashSet<JobUuid>,
    }

    impl MockWorker {
        fn with_next_job(job: PoppedJob) -> Self {
            Self {
                state: Arc::new(Mutex::new(MockState {
                    jobs: vec![job],
                    finished_jobs: HashSet::new(),
                })),
            }
        }
        fn empty() -> Self {
            Self::default()
        }
    }

    impl JobCapability for MockWorker {
        async fn unshift_job(&self, job_kind: JobKind) -> Result<(), String> {
            let mut st = self.state.lock().await;
            st.jobs.insert(
                0,
                PoppedJob {
                    uuid: JobUuid::new(),
                    kind: job_kind,
                },
            );
            Ok(())
        }
        async fn pop_next_job(&self) -> Result<Option<PoppedJob>, String> {
            let mut st = self.state.lock().await;
            Ok(st.jobs.pop())
        }
        async fn recent_jobs(&self, _limit: i64) -> Result<Vec<crate::domain::job::Job>, String> {
            Ok(vec![])
        }
        async fn mark_job_finished(&self, job_uuid: &JobUuid) -> Result<(), String> {
            let mut st = self.state.lock().await;

            st.finished_jobs.insert(job_uuid.clone());

            Ok(())
        }
    }

    #[tokio::test]
    async fn returns_ok_when_no_job_available() {
        let mock = MockWorker::empty();
        let res = run_next_job(mock.clone()).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn processes_ping_and_marks_finished() {
        let job_uuid = JobUuid::test_id(0);
        let popped = PoppedJob {
            uuid: job_uuid.clone(),
            kind: JobKind::Ping,
        };
        let mock = MockWorker::with_next_job(popped);
        let res = run_next_job(mock.clone()).await;
        assert!(res.is_ok());
        let st = mock.state.lock().await;
        assert!(st.finished_jobs.contains(&job_uuid));
    }
}
