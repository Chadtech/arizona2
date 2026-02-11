use crate::capability::event::EventCapability;
use crate::capability::job::JobCapability;
use crate::capability::memory::MemoryCapability;
use crate::capability::message::MessageCapability;
use crate::capability::person::PersonCapability;
use crate::capability::person_identity::PersonIdentityCapability;
use crate::capability::reaction::ReactionCapability;
use crate::capability::scene::SceneCapability;
use crate::capability::state_of_mind::StateOfMindCapability;
use crate::domain::job::{
    person_waiting, process_message, send_message_to_scene, JobKind, PoppedJob,
};
use crate::domain::job_uuid::JobUuid;
use crate::domain::random_seed::RandomSeed;
use crate::nice_display::NiceDisplay;
use crate::worker;
use crate::worker::Worker;
use sqlx::Row;
use std::time::Instant;

pub enum Error {
    WorkerInitError(worker::InitError),
    ActiveClockError(String),
    PopJobError(String),
    RunJobError((JobUuid, RunJobError)),
}

#[derive(Debug, Clone)]
pub enum RunNextJobResult {
    NoJob,
    RanJob { job_uuid: JobUuid, job_kind: String },
    Deferred { job_uuid: JobUuid, job_kind: String },
}

pub enum RunJobError {
    FailedToMarkJobFinished(String),
    FailedToMarkJobFailed(String),
    FailedToResetJob(String),
    ProcessMessageError(process_message::Error),
    SendMessageToSceneError(send_message_to_scene::Error),
    PersonWaitingError(person_waiting::Error),
}

enum RunJobOutcome {
    Completed,
    Deferred,
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::WorkerInitError(err) => {
                format!("Worker initialization error\n{}", err.message())
            }
            Error::ActiveClockError(err) => {
                format!("Active clock error\n{}", err)
            }
            Error::RunJobError(err) => err.message(),
            Error::PopJobError(err) => {
                format!("Failed to pop next job\n{}", err)
            }
        }
    }
}

struct ActiveClock {
    base_active_ms: i64,
    start: Instant,
}

impl ActiveClock {
    async fn load(worker: &Worker) -> Result<Self, String> {
        let row = sqlx::query(
            r#"
                SELECT active_ms
                FROM active_clock
                WHERE id = TRUE
            "#,
        )
        .fetch_optional(&worker.sqlx)
        .await
        .map_err(|err| format!("Error loading active clock: {}", err))?;

        let base_active_ms = match row {
            Some(row) => row
                .try_get::<i64, _>("active_ms")
                .map_err(|err| format!("Error reading active_ms: {}", err))?,
            None => 0,
        };

        Ok(Self {
            base_active_ms,
            start: Instant::now(),
        })
    }

    fn current_ms(&self) -> i64 {
        let elapsed = self.start.elapsed();
        let elapsed_ms = i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX);
        self.base_active_ms.saturating_add(elapsed_ms)
    }

    async fn persist(&self, worker: &Worker) -> Result<(), String> {
        sqlx::query(
            r#"
                UPDATE active_clock
                SET active_ms = $1
                WHERE id = TRUE
            "#,
        )
        .bind(self.current_ms())
        .execute(&worker.sqlx)
        .await
        .map_err(|err| format!("Error storing active clock: {}", err))?;

        Ok(())
    }
}

impl NiceDisplay for (JobUuid, RunJobError) {
    fn message(&self) -> String {
        let (job_uuid, err) = self;

        let err_msg = err.to_nice_error().to_string();

        format!("I failed to run job {}\n{}", job_uuid, err_msg)
    }
}

impl NiceDisplay for RunJobError {
    fn message(&self) -> String {
        match self {
            RunJobError::FailedToMarkJobFinished(err) => {
                format!(
                    "I ran into the following problem trying to mark the job as finished\n{}",
                    err
                )
            }
            RunJobError::ProcessMessageError(err) => {
                format!("Error processing message job\n{}", err.message())
            }
            RunJobError::SendMessageToSceneError(err) => {
                format!("Error sending message to scene job\n{}", err.message())
            }
            RunJobError::FailedToMarkJobFailed(err) => {
                format!(
                    "I ran into the following problem trying to mark the job as failed\n{}",
                    err
                )
            }
            RunJobError::FailedToResetJob(err) => {
                format!(
                    "I ran into the following problem trying to reset the job\n{}",
                    err
                )
            }
            RunJobError::PersonWaitingError(err) => {
                format!("Error processing person waiting job\n{}", err.message())
            }
        }
    }
}
pub async fn run() -> Result<(), Error> {
    let worker = Worker::new().await.map_err(Error::WorkerInitError)?;
    let active_clock = ActiveClock::load(&worker)
        .await
        .map_err(Error::ActiveClockError)?;
    tracing::info!("Job runner started, polling for jobs");
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);
    loop {
        let random_seed = match worker.get_random_seed() {
            Ok(seed) => seed,
            Err(err) => {
                tracing::error!("Job runner seed error: {}", err);
                continue;
            }
        };
        let current_active_ms = active_clock.current_ms();
        let job_fut = run_next_job(worker.clone(), random_seed, current_active_ms);

        tokio::select! {
            _ = &mut shutdown => {
                if let Err(err) = active_clock.persist(&worker).await {
                    tracing::error!("Job runner active clock error: {}", err);
                }
                tracing::info!("Job runner shutting down");
                break;
            }
            res = job_fut => {
                if let Err(err) = res {
                    // Log the error but continue processing other jobs
                    tracing::error!("Job runner error: {}", err.to_nice_error().to_string());
                }
            }
        }
    }
    Ok(())
}

pub async fn run_one_job(
    worker: Worker,
    random_seed: RandomSeed,
) -> Result<RunNextJobResult, Error> {
    let active_clock = ActiveClock::load(&worker)
        .await
        .map_err(Error::ActiveClockError)?;
    let current_active_ms = active_clock.current_ms();

    let job = match worker
        .pop_next_job(current_active_ms)
        .await
        .map_err(Error::PopJobError)? {
        Some(j) => j,
        None => {
            return Ok(RunNextJobResult::NoJob);
        }
    };

    let job_uuid = job.uuid.clone();
    let job_kind = job.kind.to_name();
    tracing::info!("Processing job {} of type {:?}", job_uuid, job.kind);

    let outcome = run_job(worker.clone(), random_seed, current_active_ms, job)
        .await
        .map_err(|err| Error::RunJobError((job_uuid.clone(), err)))?;

    active_clock
        .persist(&worker)
        .await
        .map_err(Error::ActiveClockError)?;

    match outcome {
        RunJobOutcome::Completed => Ok(RunNextJobResult::RanJob { job_uuid, job_kind }),
        RunJobOutcome::Deferred => Ok(RunNextJobResult::Deferred { job_uuid, job_kind }),
    }
}

async fn run_next_job<
    W: JobCapability
        + MessageCapability
        + SceneCapability
        + ReactionCapability
        + MemoryCapability
        + PersonCapability
        + EventCapability
        + StateOfMindCapability
        + PersonIdentityCapability,
>(
    worker: W,
    random_seed: RandomSeed,
    current_active_ms: i64,
) -> Result<(), Error> {
    let job = match worker
        .pop_next_job(current_active_ms)
        .await
        .map_err(Error::PopJobError)? {
        Some(j) => j,
        None => {
            return Ok(());
        }
    };

    let job_uuid = job.uuid.clone();
    tracing::info!("Processing job {} of type {:?}", job_uuid, job.kind);

    match run_job(worker, random_seed, current_active_ms, job)
        .await
        .map_err(|err| Error::RunJobError((job_uuid, err)))? {
        RunJobOutcome::Completed => Ok(()),
        RunJobOutcome::Deferred => Ok(()),
    }
}

async fn run_job<
    W: JobCapability
        + MessageCapability
        + SceneCapability
        + ReactionCapability
        + MemoryCapability
        + PersonCapability
        + EventCapability
        + StateOfMindCapability
        + PersonIdentityCapability,
>(
    worker: W,
    random_seed: RandomSeed,
    current_active_ms: i64,
    job: PoppedJob,
) -> Result<RunJobOutcome, RunJobError> {
    let res: Result<RunJobOutcome, RunJobError> = match job.kind {
        JobKind::Ping => {
            tracing::debug!("Ping job received");
            println!("Pong");
            Ok(RunJobOutcome::Completed)
        }
        JobKind::SendMessageToScene(job_data) => {
            tracing::debug!("Executing SendMessageToScene job");
            job_data
                .run(&worker)
                .await
                .map_err(RunJobError::SendMessageToSceneError)
                .map(|_| RunJobOutcome::Completed)
        }
        JobKind::ProcessMessage(process_message_job) => {
            tracing::debug!("Executing ProcessMessage job");
            process_message_job
                .run(&worker, random_seed, current_active_ms)
                .await
                .map_err(RunJobError::ProcessMessageError)
                .map(|_| RunJobOutcome::Completed)
        }
        JobKind::PersonWaiting(person_waiting_job) => {
            tracing::debug!("Executing PersonWaiting job");
            match person_waiting_job
                .run(&worker, random_seed, current_active_ms)
                .await
                .map_err(RunJobError::PersonWaitingError)?
            {
                person_waiting::WaitOutcome::Ready => Ok(RunJobOutcome::Completed),
                person_waiting::WaitOutcome::NotReady => {
                    worker
                        .reset_job(&job.uuid)
                        .await
                        .map_err(RunJobError::FailedToResetJob)?;
                    Ok(RunJobOutcome::Deferred)
                }
            }
        }
    };

    match res {
        Ok(RunJobOutcome::Completed) => {
            tracing::info!("Job {} completed successfully", job.uuid);
            worker
                .mark_job_finished(&job.uuid)
                .await
                .map_err(RunJobError::FailedToMarkJobFinished)?;
            Ok(RunJobOutcome::Completed)
        }
        Ok(RunJobOutcome::Deferred) => Ok(RunJobOutcome::Deferred),
        Err(ref err) => {
            tracing::error!(
                "Job {} failed: {}",
                job.uuid,
                err.to_nice_error().to_string()
            );
            worker
                .mark_job_failed(&job.uuid, err.to_nice_error().to_string().as_str())
                .await
                .map_err(RunJobError::FailedToMarkJobFailed)?;
            Ok(RunJobOutcome::Completed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::job::JobCapability;
    use crate::capability::message::{MessageCapability, NewMessage};
    use crate::capability::scene::{
        CurrentScene, NewScene, NewSceneSnapshot, Scene, SceneCapability, SceneParticipant,
    };
    use crate::domain::job::{JobKind, PoppedJob};
    use crate::domain::job_uuid::JobUuid;
    use crate::domain::message::{Message, MessageSender};
    use crate::domain::message_uuid::MessageUuid;
    use crate::domain::person_name::PersonName;
    use crate::domain::person_uuid::PersonUuid;
    use crate::domain::scene_participant_uuid::SceneParticipantUuid;
    use crate::domain::scene_uuid::SceneUuid;
    use async_trait::async_trait;
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
        sent_messages: Vec<NewMessage>,
    }

    impl MockWorker {
        fn with_next_job(job: PoppedJob) -> Self {
            Self {
                state: Arc::new(Mutex::new(MockState {
                    jobs: vec![job],
                    finished_jobs: HashSet::new(),
                    sent_messages: Vec::new(),
                })),
            }
        }
        fn empty() -> Self {
            Self::default()
        }
    }

    impl MessageCapability for MockWorker {
        async fn send_message(&self, new_message: NewMessage) -> Result<MessageUuid, String> {
            let mut st = self.state.lock().await;
            st.sent_messages.push(new_message);
            Ok(MessageUuid::new())
        }

        async fn send_scene_message(
            &self,
            _sender: MessageSender,
            _scene_uuid: SceneUuid,
            _content: String,
        ) -> Result<MessageUuid, String> {
            Ok(MessageUuid::new())
        }

        async fn add_scene_message_recipients(
            &self,
            _message_uuid: &MessageUuid,
            _recipients: Vec<PersonUuid>,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn get_messages_in_scene(
            &self,
            _scene_uuid: &SceneUuid,
        ) -> Result<Vec<Message>, String> {
            Ok(vec![])
        }

        async fn get_message_by_uuid(
            &self,
            _message_uuid: &MessageUuid,
        ) -> Result<Option<Message>, String> {
            Ok(None)
        }

        async fn mark_message_read(&self, _message_uuid: &MessageUuid) -> Result<(), String> {
            Ok(())
        }

        async fn get_unhandled_scene_messages_for_person(
            &self,
            _person_uuid: &PersonUuid,
            _scene_uuid: &SceneUuid,
        ) -> Result<Vec<Message>, String> {
            Ok(vec![])
        }

        async fn mark_scene_messages_handled_for_person(
            &self,
            _person_uuid: &PersonUuid,
            _message_uuids: Vec<MessageUuid>,
        ) -> Result<(), String> {
            Ok(())
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
        async fn pop_next_job(&self, _current_active_ms: i64) -> Result<Option<PoppedJob>, String> {
            let mut st = self.state.lock().await;
            Ok(st.jobs.pop())
        }
        async fn recent_jobs(&self, _limit: i64) -> Result<Vec<crate::domain::job::Job>, String> {
            Ok(vec![])
        }
        async fn get_job_by_uuid(
            &self,
            _job_uuid: &JobUuid,
        ) -> Result<Option<crate::domain::job::Job>, String> {
            Ok(None)
        }
        async fn mark_job_finished(&self, job_uuid: &JobUuid) -> Result<(), String> {
            let mut st = self.state.lock().await;

            st.finished_jobs.insert(job_uuid.clone());

            Ok(())
        }

        async fn mark_job_failed(&self, _job_uuid: &JobUuid, _details: &str) -> Result<(), String> {
            Ok(())
        }

        async fn reset_job(&self, _job_uuid: &JobUuid) -> Result<(), String> {
            Ok(())
        }

        async fn delete_job(&self, _job_uuid: &JobUuid) -> Result<(), String> {
            Ok(())
        }
    }

    #[async_trait]
    impl SceneCapability for MockWorker {
        async fn create_scene(&self, _new_scene: NewScene) -> Result<SceneUuid, String> {
            Ok(SceneUuid::new())
        }

        async fn add_person_to_scene(
            &self,
            _scene_uuid: SceneUuid,
            _person_name: PersonName,
        ) -> Result<SceneParticipantUuid, String> {
            Ok(SceneParticipantUuid::new())
        }

        async fn remove_person_from_scene(
            &self,
            _scene_uuid: SceneUuid,
            _person_name: PersonName,
        ) -> Result<SceneParticipantUuid, String> {
            Ok(SceneParticipantUuid::new())
        }

        async fn get_persons_current_scene(
            &self,
            _person_name: PersonName,
        ) -> Result<Option<CurrentScene>, String> {
            Ok(None)
        }

        async fn create_scene_snapshot(
            &self,
            _new_scene_snapshot: NewSceneSnapshot,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn get_scene_from_name(&self, _scene_name: String) -> Result<Option<Scene>, String> {
            Ok(None)
        }

        async fn get_scene_current_participants(
            &self,
            _scene_uuid: &SceneUuid,
        ) -> Result<Vec<SceneParticipant>, String> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn returns_ok_when_no_job_available() {
        let mock = MockWorker::empty();
        let res = run_next_job(mock.clone(), RandomSeed::from_u64(0), 0).await;
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
        let res = run_next_job(mock.clone(), RandomSeed::from_u64(0), 0).await;
        assert!(res.is_ok());
        let st = mock.state.lock().await;
        assert!(st.finished_jobs.contains(&job_uuid));
    }
}
