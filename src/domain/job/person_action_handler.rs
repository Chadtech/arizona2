use crate::capability::job::JobCapability;
use crate::capability::message::MessageCapability;
use crate::capability::scene::SceneCapability;
use crate::capability::person::PersonCapability;
use crate::domain::job::person_waiting::PersonWaitingJob;
use crate::domain::job::send_message_to_scene::send_scene_message_and_enqueue_recipients;
use crate::domain::job::JobKind;
use crate::domain::message::MessageSender;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::random_seed::RandomSeed;
use crate::domain::scene_uuid::SceneUuid;
use crate::nice_display::NiceDisplay;
use crate::person_actions::PersonAction;

pub enum ActionHandleError {
    Wait(String),
    SceneMissing(String),
    Say {
        scene_uuid: SceneUuid,
        details: String,
    },
}

impl NiceDisplay for ActionHandleError {
    fn message(&self) -> String {
        match self {
            ActionHandleError::Wait(details) => {
                format!("Person could not wait: {}", details)
            }
            ActionHandleError::SceneMissing(details) => {
                format!("Could not get person's scene: {}", details)
            }
            ActionHandleError::Say {
                scene_uuid,
                details,
            } => {
                format!(
                    "Person could not say in scene {}: {}",
                    scene_uuid.to_uuid(),
                    details
                )
            }
        }
    }
}

const IDLE_DURATION_MS: i64 = 4 * 60 * 1000;

pub async fn handle_person_action<
    W: SceneCapability + JobCapability + PersonCapability + MessageCapability,
>(
    worker: &W,
    action: &PersonAction,
    person_uuid: &PersonUuid,
    random_seed: RandomSeed,
    current_active_ms: i64,
) -> Result<(), ActionHandleError> {
    match action {
        PersonAction::Wait { duration } => {
            enqueue_wait(worker, person_uuid, *duration, current_active_ms).await
        }
        PersonAction::Idle => {
            enqueue_wait(
                worker,
                person_uuid,
                IDLE_DURATION_MS as u64,
                current_active_ms,
            )
            .await
        }
        PersonAction::SayInScene { comment } => {
            let sender = MessageSender::AiPerson(person_uuid.clone());

            let scene_uuid = worker
                .get_persons_current_scene_uuid(person_uuid)
                .await
                .map_err(ActionHandleError::SceneMissing)?
                .ok_or_else(|| ActionHandleError::SceneMissing("Person is not in any scene".to_string()))?;

            send_scene_message_and_enqueue_recipients(
                worker,
                sender,
                scene_uuid.clone(),
                comment.clone(),
                random_seed.clone(),
            )
            .await
            .map_err(|err| ActionHandleError::Say {
                scene_uuid,
                details: err.to_nice_error().to_string(),
            })?;

            Ok(())
        }
    }
}

async fn enqueue_wait<
    W: JobCapability,
>(
    worker: &W,
    person_uuid: &PersonUuid,
    duration_ms: u64,
    current_active_ms: i64,
) -> Result<(), ActionHandleError> {
    let duration_i64: i64 = duration_ms.min(i64::MAX as u64) as i64;
    let person_waiting_job = PersonWaitingJob::new(
        person_uuid.clone(),
        duration_i64,
        current_active_ms,
    );
    let wait_job = JobKind::PersonWaiting(person_waiting_job);
    worker
        .unshift_job(wait_job)
        .await
        .map_err(ActionHandleError::Wait)?;
    Ok(())
}
