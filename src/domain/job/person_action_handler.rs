use crate::capability::job::JobCapability;
use crate::capability::logging::LogCapability;
use crate::capability::message::MessageCapability;
use crate::capability::person::PersonCapability;
use crate::capability::reaction_history::ReactionHistoryCapability;
use crate::capability::scene::SceneCapability;
use crate::domain::actor_uuid::ActorUuid;
use crate::domain::job::person_hibernating::PersonHibernatingJob;
use crate::domain::job::person_waiting::PersonWaitingJob;
use crate::domain::job::process_person_join::ProcessPersonJoinJob;
use crate::domain::job::send_message_to_scene::send_scene_message_and_enqueue_recipients;
use crate::domain::job::JobKind;
use crate::domain::logger::Level;
use crate::domain::message::MessageSender;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::random_seed::RandomSeed;
use crate::domain::scene_uuid::SceneUuid;
use crate::nice_display::NiceDisplay;
use crate::person_actions::PersonAction;

pub enum ActionHandleError {
    Wait(String),
    Hibernate(String),
    HibernationState(String),
    ReactionLog(String),
    PersonName(String),
    SceneMissing(String),
    Say {
        scene_uuid: SceneUuid,
        details: String,
    },
    MoveToScene(String),
}

impl NiceDisplay for ActionHandleError {
    fn message(&self) -> String {
        match self {
            ActionHandleError::Wait(details) => {
                format!("Person could not wait: {}", details)
            }
            ActionHandleError::Hibernate(details) => {
                format!("Person could not hibernate: {}", details)
            }
            ActionHandleError::HibernationState(details) => {
                format!("Could not set hibernation state: {}", details)
            }
            ActionHandleError::ReactionLog(details) => {
                format!("Could not record reaction: {}", details)
            }
            ActionHandleError::PersonName(details) => {
                format!("Could not get person's name: {}", details)
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
            ActionHandleError::MoveToScene(details) => {
                format!("Person could not move to scene: {}", details)
            }
        }
    }
}

const IDLE_DURATION_MS: i64 = 4 * 60 * 1000;
const POST_MOVE_WAIT_MS: u64 = 30 * 1000;

pub async fn handle_person_action<
    W: SceneCapability
        + JobCapability
        + PersonCapability
        + MessageCapability
        + ReactionHistoryCapability
        + LogCapability
        + Sync,
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
        PersonAction::Hibernate { duration } => {
            worker
                .set_person_hibernating(person_uuid, true)
                .await
                .map_err(ActionHandleError::HibernationState)?;
            enqueue_hibernation(worker, person_uuid, *duration, current_active_ms).await
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
        PersonAction::SayInScene {
            comment,
            destination_scene_name,
        } => {
            let sender = MessageSender::AiPerson(person_uuid.clone());
            let person_name = worker
                .get_persons_name(person_uuid.clone())
                .await
                .map_err(ActionHandleError::PersonName)?;

            let scene_uuid = worker
                .get_persons_current_scene_uuid(person_uuid)
                .await
                .map_err(ActionHandleError::SceneMissing)?
                .ok_or_else(|| {
                    ActionHandleError::SceneMissing("Person is not in any scene".to_string())
                })?;

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

            let person_label = person_name.to_string();
            worker.log(
                Level::Info,
                format!("AI person {} said in scene: {}", person_label, comment).as_str(),
            );

            if let Some(destination_scene_name) = destination_scene_name {
                move_person_to_scene(
                    worker,
                    person_uuid,
                    destination_scene_name,
                    current_active_ms,
                )
                .await?;
            }

            let reaction_kind = match destination_scene_name {
                Some(_) => "say_in_scene_and_move_to_scene",
                None => "say_in_scene",
            };
            worker
                .record_reaction(person_uuid, reaction_kind)
                .await
                .map_err(ActionHandleError::ReactionLog)?;

            Ok(())
        }
        PersonAction::MoveToScene { scene_name } => {
            move_person_to_scene(worker, person_uuid, scene_name, current_active_ms).await?;

            worker
                .record_reaction(person_uuid, "move_to_scene")
                .await
                .map_err(ActionHandleError::ReactionLog)?;

            Ok(())
        }
    }
}

async fn enqueue_wait<W: JobCapability>(
    worker: &W,
    person_uuid: &PersonUuid,
    duration_ms: u64,
    current_active_ms: i64,
) -> Result<(), ActionHandleError> {
    let duration_i64: i64 = duration_ms.min(i64::MAX as u64) as i64;
    let person_waiting_job =
        PersonWaitingJob::new(person_uuid.clone(), duration_i64, current_active_ms);
    let wait_job = JobKind::PersonWaiting(person_waiting_job);
    worker
        .unshift_job(wait_job)
        .await
        .map_err(ActionHandleError::Wait)?;
    Ok(())
}

async fn move_person_to_scene<
    W: SceneCapability
        + JobCapability
        + PersonCapability
        + MessageCapability
        + ReactionHistoryCapability
        + LogCapability
        + Sync,
>(
    worker: &W,
    person_uuid: &PersonUuid,
    scene_name: &str,
    current_active_ms: i64,
) -> Result<(), ActionHandleError> {
    let person_name = worker
        .get_persons_name(person_uuid.clone())
        .await
        .map_err(ActionHandleError::PersonName)?;

    let from_scene_uuid = worker
        .get_persons_current_scene_uuid(person_uuid)
        .await
        .map_err(ActionHandleError::SceneMissing)?;

    let maybe_scene = worker
        .get_scene_from_name(scene_name.to_string())
        .await
        .map_err(ActionHandleError::MoveToScene)?;

    let scene = match maybe_scene {
        Some(scene) => scene,
        None => {
            let basis_scene_uuid = from_scene_uuid.clone().ok_or_else(|| {
                ActionHandleError::MoveToScene(
                    "Person is not currently in a scene; cannot derive travel context".to_string(),
                )
            })?;

            let created_scene = worker
                .create_scene_from_travel(scene_name.to_string(), basis_scene_uuid)
                .await
                .map_err(ActionHandleError::MoveToScene)?;

            worker.log(
                Level::Info,
                format!(
                    "Created destination scene '{}' on the fly for movement",
                    scene_name
                )
                .as_str(),
            );

            created_scene
        }
    };

    if from_scene_uuid
        .as_ref()
        .map(|uuid| uuid.to_uuid())
        == Some(scene.uuid.to_uuid())
    {
        return Ok(());
    }

    let from_scene_desc = match &from_scene_uuid {
        Some(uuid) => uuid.to_uuid().to_string(),
        None => "none".to_string(),
    };
    worker.log(
        Level::Info,
        format!(
            "AI person {} moving from scene {} to scene: {}",
            person_name.as_str(),
            from_scene_desc,
            scene_name
        )
        .as_str(),
    );

    worker
        .add_person_to_scene(scene.uuid.clone(), person_name.clone())
        .await
        .map_err(ActionHandleError::MoveToScene)?;

    let person_label = person_name.to_string();
    worker.log(
        Level::Info,
        format!(
            "AI person {} moved to scene {} ({})",
            person_label,
            scene_name,
            scene.uuid.to_uuid()
        )
        .as_str(),
    );

    enqueue_wait(worker, person_uuid, POST_MOVE_WAIT_MS, current_active_ms).await?;
    enqueue_person_join_jobs(worker, &scene.uuid, person_uuid).await?;

    Ok(())
}

async fn enqueue_person_join_jobs<W: SceneCapability + JobCapability>(
    worker: &W,
    scene_uuid: &SceneUuid,
    joined_person_uuid: &PersonUuid,
) -> Result<(), ActionHandleError> {
    let participants = worker
        .get_scene_current_participants(scene_uuid)
        .await
        .map_err(ActionHandleError::MoveToScene)?;

    for participant in participants {
        match participant.actor_uuid {
            ActorUuid::AiPerson(recipient_person_uuid) => {
                let job = ProcessPersonJoinJob {
                    scene_uuid: scene_uuid.clone(),
                    joined_person_uuid: joined_person_uuid.clone(),
                    recipient_person_uuid,
                };

                worker
                    .unshift_job(JobKind::ProcessPersonJoin(job))
                    .await
                    .map_err(ActionHandleError::MoveToScene)?;
            }
            ActorUuid::RealWorldUser => {}
        }
    }

    Ok(())
}

async fn enqueue_hibernation<W: JobCapability>(
    worker: &W,
    person_uuid: &PersonUuid,
    duration_ms: u64,
    current_active_ms: i64,
) -> Result<(), ActionHandleError> {
    let duration_i64: i64 = duration_ms.min(i64::MAX as u64) as i64;
    let person_hibernating_job =
        PersonHibernatingJob::new(person_uuid.clone(), duration_i64, current_active_ms);
    let hibernation_job = JobKind::PersonHibernating(person_hibernating_job);
    worker
        .unshift_job(hibernation_job)
        .await
        .map_err(ActionHandleError::Hibernate)?;
    Ok(())
}
