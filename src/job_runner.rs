use crate::capability::event::EventCapability;
use crate::capability::job::JobCapability;
use crate::capability::job_runner_settings::JobRunnerSettingsCapability;
use crate::capability::log_event::LogEventCapability;
use crate::capability::logging::LogCapability;
use crate::capability::memory::MemoryCapability;
use crate::capability::message::MessageCapability;
use crate::capability::motivation::MotivationCapability;
use crate::capability::person::PersonCapability;
use crate::capability::person_identity::PersonIdentityCapability;
use crate::capability::reaction::ReactionCapability;
use crate::capability::reaction_history::ReactionHistoryCapability;
use crate::capability::reflection::ReflectionCapability;
use crate::capability::scene::SceneCapability;
use crate::capability::state_of_mind::StateOfMindCapability;
use crate::domain::job::{
    person_hibernating, person_waiting, process_message, process_person_join,
    send_message_to_scene, JobKind, PoppedJob,
};
use crate::domain::job_uuid::JobUuid;
use crate::domain::logger::{Level, Logger};
use crate::domain::random_seed::RandomSeed;
use crate::nice_display::NiceDisplay;
use crate::worker;
use crate::worker::Worker;
use sqlx::Row;
use std::time::Instant;

const DEFAULT_JOB_RUNNER_POLL_INTERVAL_SECS: u64 = 45;

pub enum Error {
    WorkerInit(worker::InitError),
    ActiveClock(String),
    PopJob(String),
    RunJob((JobUuid, RunJobError)),
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
    ProcessPersonJoinError(process_person_join::Error),
    SendMessageToSceneError(send_message_to_scene::Error),
    PersonWaitingError(person_waiting::Error),
    PersonHibernatingError(person_hibernating::Error),
}

enum RunJobOutcome {
    Completed,
    Deferred,
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::WorkerInit(err) => {
                format!("Worker initialization error\n{}", err.message())
            }
            Error::ActiveClock(err) => {
                format!("Active clock error\n{}", err)
            }
            Error::RunJob(err) => err.message(),
            Error::PopJob(err) => {
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
            RunJobError::ProcessPersonJoinError(err) => {
                format!("Error processing person join job\n{}", err.message())
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
            RunJobError::PersonHibernatingError(err) => {
                format!("Error processing person hibernating job\n{}", err.message())
            }
        }
    }
}
pub async fn run() -> Result<(), Error> {
    let logger = Logger::init(Level::Info).log_to_file();

    let worker = Worker::new(logger).await.map_err(Error::WorkerInit)?;
    let active_clock = ActiveClock::load(&worker)
        .await
        .map_err(Error::ActiveClock)?;
    tracing::info!("Job runner started, polling for jobs");
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);
    loop {
        let poll_interval_secs = match worker.get_job_runner_poll_interval_secs().await {
            Ok(secs) => secs,
            Err(err) => {
                tracing::error!("Job runner poll interval error: {}", err);
                DEFAULT_JOB_RUNNER_POLL_INTERVAL_SECS
            }
        };

        let job_runner_enabled = match worker.get_job_runner_enabled().await {
            Ok(enabled) => enabled,
            Err(err) => {
                tracing::error!("Job runner enabled flag error: {}", err);
                true
            }
        };

        if job_runner_enabled {
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
                        let err_message = err.to_nice_error().to_string();
                        tracing::error!("Job runner error: {}", err_message);
                        worker
                            .logger
                            .log(Level::Error, &format!("Job runner error: {}", err_message));
                    }
                }
            }
        }

        tokio::select! {
            _ = &mut shutdown => {
                if let Err(err) = active_clock.persist(&worker).await {
                    tracing::error!("Job runner active clock error: {}", err);
                }
                tracing::info!("Job runner shutting down");
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(poll_interval_secs)) => {}
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
        .map_err(Error::ActiveClock)?;
    let current_active_ms = active_clock.current_ms();

    let job = match worker
        .pop_next_job(current_active_ms)
        .await
        .map_err(Error::PopJob)?
    {
        Some(j) => j,
        None => {
            return Ok(RunNextJobResult::NoJob);
        }
    };

    let job_uuid = job.uuid.clone();
    let job_kind = job.kind.to_name();
    tracing::info!("Processing job {} of type {:?}", job_uuid, job.kind);

    let outcome = match run_job(worker.clone(), random_seed, current_active_ms, job).await {
        Ok(outcome) => outcome,
        Err(err) => {
            let err_message = err.to_nice_error().to_string();
            worker
                .logger
                .log(Level::Error, &format!("Job runner error: {}", err_message));
            return Err(Error::RunJob((job_uuid.clone(), err)));
        }
    };

    active_clock
        .persist(&worker)
        .await
        .map_err(Error::ActiveClock)?;

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
        + PersonIdentityCapability
        + ReactionHistoryCapability
        + LogEventCapability
        + ReflectionCapability
        + MotivationCapability
        + LogCapability
        + Sync,
>(
    worker: W,
    random_seed: RandomSeed,
    current_active_ms: i64,
) -> Result<(), Error> {
    let job = match worker
        .pop_next_job(current_active_ms)
        .await
        .map_err(Error::PopJob)?
    {
        Some(j) => j,
        None => {
            return Ok(());
        }
    };

    let job_uuid = job.uuid.clone();
    tracing::info!("Processing job {} of type {:?}", job_uuid, job.kind);

    match run_job(worker, random_seed, current_active_ms, job)
        .await
        .map_err(|err| Error::RunJob((job_uuid, err)))?
    {
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
        + PersonIdentityCapability
        + ReactionHistoryCapability
        + LogEventCapability
        + ReflectionCapability
        + MotivationCapability
        + LogCapability
        + Sync,
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
        JobKind::ProcessPersonJoin(process_person_join_job) => {
            tracing::debug!("Executing ProcessPersonJoin job");
            process_person_join_job
                .run(&worker, random_seed, current_active_ms)
                .await
                .map_err(RunJobError::ProcessPersonJoinError)
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
        JobKind::PersonHibernating(person_hibernating_job) => {
            tracing::debug!("Executing PersonHibernating job");
            match person_hibernating_job
                .run(&worker, current_active_ms)
                .await
                .map_err(RunJobError::PersonHibernatingError)?
            {
                person_hibernating::HibernateOutcome::Ready => Ok(RunJobOutcome::Completed),
                person_hibernating::HibernateOutcome::NotReady => {
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
    use crate::capability::event::{EventCapability, GetArgs};
    use crate::capability::job::JobCapability;
    use crate::capability::log_event::LogEventCapability;
    use crate::capability::logging::LogCapability;
    use crate::capability::memory::{
        MemoryCapability, MemoryQueryPrompt, MemorySearchResult, MessageTypeArgs, NewMemory,
    };
    use crate::capability::message::MessageCapability;
    use crate::capability::motivation::{MotivationCapability, NewMotivation};
    use crate::capability::person::{NewPerson, PersonCapability};
    use crate::capability::person_identity::{NewPersonIdentity, PersonIdentityCapability};
    use crate::capability::reaction::ReactionCapability;
    use crate::capability::reaction_history::ReactionHistoryCapability;
    use crate::capability::reflection::{ReflectionCapability, ReflectionChange};
    use crate::capability::scene::{
        CurrentScene, NewScene, NewSceneSnapshot, Scene, SceneCapability, SceneParticipant,
        SceneParticipation,
    };
    use crate::capability::state_of_mind::{NewStateOfMind, StateOfMindCapability};
    use crate::domain::job::{JobKind, PoppedJob};
    use crate::domain::job_uuid::JobUuid;
    use crate::domain::logger::Level;
    use crate::domain::memory::Memory;
    use crate::domain::memory_uuid::MemoryUuid;
    use crate::domain::message::{Message, MessageSender};
    use crate::domain::message_uuid::MessageUuid;
    use crate::domain::motivation::Motivation;
    use crate::domain::motivation_uuid::MotivationUuid;
    use crate::domain::person_identity_uuid::PersonIdentityUuid;
    use crate::domain::person_name::PersonName;
    use crate::domain::person_uuid::PersonUuid;
    use crate::domain::scene_participant_uuid::SceneParticipantUuid;
    use crate::domain::scene_uuid::SceneUuid;
    use crate::domain::state_of_mind::StateOfMind;
    use crate::domain::state_of_mind_uuid::StateOfMindUuid;
    use crate::person_actions::{PersonAction, PersonReaction, ReflectionDecision};
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
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

    impl MessageCapability for MockWorker {
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

        async fn get_messages_in_scene_page(
            &self,
            _scene_uuid: &SceneUuid,
            _limit: i64,
            _before_sent_at: Option<chrono::DateTime<chrono::Utc>>,
        ) -> Result<Vec<Message>, String> {
            Ok(vec![])
        }

        async fn get_message_by_uuid(
            &self,
            _message_uuid: &MessageUuid,
        ) -> Result<Option<Message>, String> {
            Ok(None)
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

        async fn reset_all_failed_jobs(&self) -> Result<(), String> {
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

        async fn delete_scene(&self, _scene_uuid: &SceneUuid) -> Result<(), String> {
            Ok(())
        }

        async fn get_scenes(&self) -> Result<Vec<Scene>, String> {
            Ok(vec![])
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

        async fn get_persons_current_scene_uuid(
            &self,
            _person_uuid: &PersonUuid,
        ) -> Result<Option<SceneUuid>, String> {
            Ok(None)
        }

        async fn get_scene_participation_history(
            &self,
            _scene_uuid: &SceneUuid,
        ) -> Result<Vec<SceneParticipation>, String> {
            Ok(vec![])
        }

        async fn set_real_world_user_in_scene(
            &self,
            _scene_uuid: &SceneUuid,
            _is_in_scene: bool,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn is_real_world_user_in_scene(
            &self,
            _scene_uuid: &SceneUuid,
        ) -> Result<bool, String> {
            Ok(false)
        }

        async fn get_scene_name(&self, _scene_uuid: &SceneUuid) -> Result<Option<String>, String> {
            Ok(None)
        }

        async fn get_scene_description(
            &self,
            _scene_uuid: &SceneUuid,
        ) -> Result<Option<String>, String> {
            Ok(None)
        }
    }

    impl ReactionCapability for MockWorker {
        async fn get_reaction(
            &self,
            _memories: Vec<Memory>,
            _person_uuid: PersonUuid,
            _person_identity: String,
            _state_of_mind: String,
            _situation: String,
        ) -> Result<PersonReaction, String> {
            Ok(PersonReaction {
                action: PersonAction::Idle,
                reflection: ReflectionDecision::NoReflection,
            })
        }
    }

    impl ReflectionCapability for MockWorker {
        async fn get_reflection_changes(
            &self,
            _memories: Vec<Memory>,
            _person_uuid: PersonUuid,
            _person_identity: String,
            _state_of_mind: String,
            _situation: String,
        ) -> Result<Vec<ReflectionChange>, String> {
            Ok(vec![])
        }
    }

    impl ReactionHistoryCapability for MockWorker {
        async fn record_reaction(
            &self,
            _person_uuid: &PersonUuid,
            _action_kind: &str,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn has_reacted_since(
            &self,
            _person_uuid: &PersonUuid,
            _since: DateTime<Utc>,
        ) -> Result<bool, String> {
            Ok(false)
        }
    }

    impl MotivationCapability for MockWorker {
        async fn create_motivation(
            &self,
            _new_motivation: NewMotivation,
        ) -> Result<MotivationUuid, String> {
            Ok(MotivationUuid::new())
        }

        async fn get_motivations_for_person(
            &self,
            _person_uuid: PersonUuid,
        ) -> Result<Vec<Motivation>, String> {
            Ok(vec![])
        }

        async fn delete_motivation(&self, _motivation_uuid: MotivationUuid) -> Result<(), String> {
            Ok(())
        }
    }

    impl MemoryCapability for MockWorker {
        async fn create_memory(&self, _new_memory: NewMemory) -> Result<MemoryUuid, String> {
            Ok(MemoryUuid::new())
        }

        async fn maybe_create_memories_from_description(
            &self,
            _person_uuid: PersonUuid,
            _description: String,
        ) -> Result<Vec<MemoryUuid>, String> {
            Ok(vec![])
        }

        async fn create_memory_query_prompt(
            &self,
            _person_recalling: &PersonName,
            _message_type_args: MessageTypeArgs,
            _recent_events: Vec<String>,
            _state_of_mind: &String,
            _situation: &String,
        ) -> Result<MemoryQueryPrompt, String> {
            Ok(MemoryQueryPrompt {
                prompt: String::new(),
            })
        }

        async fn search_memories(
            &self,
            _person_uuid: PersonUuid,
            _query: String,
            _limit: i64,
        ) -> Result<Vec<MemorySearchResult>, String> {
            Ok(vec![])
        }
    }

    impl PersonCapability for MockWorker {
        async fn create_person(&self, _new_person: NewPerson) -> Result<PersonUuid, String> {
            Ok(PersonUuid::new())
        }

        async fn get_persons_name(&self, _person_uuid: PersonUuid) -> Result<PersonName, String> {
            Ok(PersonName::from_string("Test".to_string()))
        }

        async fn get_person_uuid_by_name(
            &self,
            _person_name: PersonName,
        ) -> Result<PersonUuid, String> {
            Ok(PersonUuid::new())
        }

        async fn set_person_hibernating(
            &self,
            _person_uuid: &PersonUuid,
            _is_hibernating: bool,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn is_person_hibernating(&self, _person_uuid: &PersonUuid) -> Result<bool, String> {
            Ok(false)
        }

        async fn set_person_enabled(
            &self,
            _person_uuid: &PersonUuid,
            _is_enabled: bool,
        ) -> Result<(), String> {
            Ok(())
        }

        async fn is_person_enabled(&self, _person_uuid: &PersonUuid) -> Result<bool, String> {
            Ok(true)
        }
    }

    impl EventCapability for MockWorker {
        async fn get_events(
            &self,
            _args: GetArgs,
        ) -> Result<Vec<crate::domain::event::Event>, String> {
            Ok(vec![])
        }
    }

    impl LogCapability for MockWorker {
        fn log(&self, _level: Level, _message: &str) {
            // no-op for tests
        }
    }

    impl LogEventCapability for MockWorker {
        async fn log_event(
            &self,
            _event_name: String,
            _data: Option<serde_json::Value>,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    #[async_trait]
    impl StateOfMindCapability for MockWorker {
        async fn create_state_of_mind(
            &self,
            _new_state_of_mind: NewStateOfMind,
        ) -> Result<StateOfMindUuid, String> {
            Ok(StateOfMindUuid::new())
        }

        async fn get_latest_state_of_mind(
            &self,
            _person_uuid: &PersonUuid,
        ) -> Result<Option<StateOfMind>, String> {
            Ok(None)
        }
    }

    #[async_trait]
    impl PersonIdentityCapability for MockWorker {
        async fn summarize_person_identity(
            &self,
            _person_name: &str,
            _identity: &str,
        ) -> Result<String, String> {
            Ok("summary".to_string())
        }

        async fn create_person_identity(
            &self,
            _new_person_identity: NewPersonIdentity,
        ) -> Result<PersonIdentityUuid, String> {
            Ok(PersonIdentityUuid::new())
        }

        async fn get_person_identity(
            &self,
            _person_uuid: &PersonUuid,
        ) -> Result<Option<String>, String> {
            Ok(None)
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
