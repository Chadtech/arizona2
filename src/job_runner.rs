use crate::capability::job::JobCapability;
use crate::capability::message::MessageCapability;
use crate::capability::scene::SceneCapability;
use crate::domain::job::{process_message, send_message_to_scene, JobKind, PoppedJob};
use crate::domain::job_uuid::JobUuid;
use crate::nice_display::NiceDisplay;
use crate::worker;
use crate::worker::Worker;

pub enum Error {
    WorkerInitError(worker::InitError),
    PopJobError(String),
    RunJobError((JobUuid, RunJobError)),
}

pub enum RunJobError {
    FailedToMarkJobFinished(String),
    FailedToMarkJobFailed(String),
    ProcessMessageError(process_message::Error),
    SendMessageToSceneError(send_message_to_scene::Error),
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::WorkerInitError(err) => {
                format!("Worker initialization error\n{}", err.message())
            }
            Error::RunJobError(err) => err.message(),
            Error::PopJobError(err) => {
                format!("Failed to pop next job\n{}", err)
            }
        }
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
        }
    }
}
pub async fn run() -> Result<(), Error> {
    let worker = Worker::new().await.map_err(Error::WorkerInitError)?;
    loop {
        run_next_job(worker.clone()).await?;
    }
}

async fn run_next_job<W: JobCapability + MessageCapability + SceneCapability>(
    worker: W,
) -> Result<(), Error> {
    let job = match worker.pop_next_job().await.map_err(Error::PopJobError)? {
        Some(j) => j,
        None => {
            return Ok(());
        }
    };

    let job_uuid = job.uuid.clone();

    run_job(worker, job)
        .await
        .map_err(|err| Error::RunJobError((job_uuid, err)))
}

async fn run_job<W: JobCapability + MessageCapability + SceneCapability>(
    worker: W,
    job: PoppedJob,
) -> Result<(), RunJobError> {
    let res: Result<(), RunJobError> = match job.kind {
        JobKind::Ping => {
            println!("Pong");
            Ok(())
        }
        JobKind::SendMessageToScene(job_data) => job_data
            .run(&worker)
            .await
            .map_err(RunJobError::SendMessageToSceneError),
        JobKind::ProcessMessage(process_message_job) => process_message_job
            .run(&worker)
            .await
            .map_err(RunJobError::ProcessMessageError),
    };

    match res {
        Ok(_) => worker
            .mark_job_finished(&job.uuid)
            .await
            .map_err(RunJobError::FailedToMarkJobFinished),
        Err(err) => worker
            .mark_job_failed(&job.uuid, err.to_nice_error().to_string().as_str())
            .await
            .map_err(RunJobError::FailedToMarkJobFailed),
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
    use crate::domain::message::Message;
    use crate::domain::message_uuid::MessageUuid;
    use crate::domain::person_name::PersonName;
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

        async fn get_scene_participants(
            &self,
            _scene_uuid: &SceneUuid,
        ) -> Result<Vec<SceneParticipant>, String> {
            Ok(vec![])
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
