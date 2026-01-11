use crate::capability::event::EventCapability;
use crate::capability::job::JobCapability;
use crate::capability::memory::{MemoryCapability, MemorySearchResult, MessageTypeArgs};
use crate::capability::person::PersonCapability;
use crate::capability::person_identity::PersonIdentityCapability;
use crate::capability::reaction::ReactionCapability;
use crate::capability::scene::SceneCapability;
use crate::capability::state_of_mind::StateOfMindCapability;
use crate::domain::job::send_message_to_scene::SendMessageToSceneJob;
use crate::domain::job::JobKind;
use crate::domain::memory::Memory;
use crate::domain::message::MessageSender;
use crate::domain::person_name::PersonName;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::random_seed::RandomSeed;
use crate::domain::scene_uuid::SceneUuid;
use crate::domain::state_of_mind::StateOfMind;
use crate::nice_display::NiceDisplay;
use crate::open_ai::completion::CompletionError;
use crate::person_actions::PersonAction;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersonWaitingJob {
    #[serde(default)]
    person_uuid: Option<PersonUuid>,
    #[serde(default)]
    started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    duration_ms: i64,
    #[serde(default)]
    start_active_ms: i64,
}

pub enum Error {
    MissingPersonUuid,
    MissingStartedAt,
    FailedToGetEvents(String),
    FailedToGetStateOfMind(String),
    NoStateOfMindFound {
        person_uuid: PersonUuid,
    },
    FailedToGetPersonIdentity(String),
    NoPersonIdentityFound {
        person_uuid: PersonUuid,
    },
    FailedToGetPersonsName(String),
    CouldNotCreateMemoriesPrompt(String),
    FailedToSearchMemories(String),
    GetPersonReactionCompletionError(CompletionError),
    CouldNotGetPersonsScene {
        person_uuid: PersonUuid,
        details: String,
    },
    PersonCouldNotWait {
        person_uuid: PersonUuid,
        error: String,
    },
    PersonCouldNotSayInScene {
        scene_uuid: SceneUuid,
        details: String,
        subject: String,
    },
}

pub enum WaitOutcome {
    Ready,
    NotReady,
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::MissingPersonUuid => "Missing person uuid on wait job".to_string(),
            Error::MissingStartedAt => "Missing started_at on wait job".to_string(),
            Error::FailedToGetEvents(err) => {
                format!("Failed to get events: {}", err)
            }
            Error::FailedToGetStateOfMind(err) => {
                format!("Failed to get state of mind: {}", err)
            }
            Error::NoStateOfMindFound { person_uuid } => {
                format!(
                    "No state of mind found for person {}",
                    person_uuid.to_uuid()
                )
            }
            Error::FailedToGetPersonIdentity(err) => {
                format!("Failed to get person identity: {}", err)
            }
            Error::NoPersonIdentityFound { person_uuid } => {
                format!(
                    "No person identity found for person {}",
                    person_uuid.to_uuid()
                )
            }
            Error::FailedToGetPersonsName(err) => {
                format!("Failed to get person's name: {}", err)
            }
            Error::CouldNotCreateMemoriesPrompt(err) => {
                format!("Could not create memories prompt: {}", err)
            }
            Error::FailedToSearchMemories(err) => {
                format!("Failed to search memories: {}", err)
            }
            Error::GetPersonReactionCompletionError(err) => {
                format!("Failed to get person reaction: {}", err.message())
            }
            Error::CouldNotGetPersonsScene {
                person_uuid,
                details,
            } => {
                format!(
                    "Could not get current scene for person {}: {}",
                    person_uuid.to_uuid(),
                    details
                )
            }
            Error::PersonCouldNotWait { person_uuid, error } => {
                format!("Person {} could not wait: {}", person_uuid.to_uuid(), error)
            }
            Error::PersonCouldNotSayInScene {
                scene_uuid,
                details,
                subject,
            } => {
                format!(
                    "Person {} could not say in scene {}: {}",
                    subject,
                    scene_uuid.to_uuid(),
                    details
                )
            }
        }
    }
}

impl PersonWaitingJob {
    pub fn new(person_uuid: PersonUuid, duration_ms: i64, start_active_ms: i64) -> Self {
        Self {
            person_uuid: Some(person_uuid),
            started_at: Some(Utc::now()),
            duration_ms: duration_ms.max(0),
            start_active_ms: start_active_ms.max(0),
        }
    }

    pub fn run_at_active_ms(&self) -> i64 {
        self.start_active_ms
            .saturating_add(self.duration_ms.max(0))
    }

    pub async fn run<
        W: JobCapability
            + SceneCapability
            + ReactionCapability
            + MemoryCapability
            + PersonCapability
            + EventCapability
            + StateOfMindCapability
            + PersonIdentityCapability,
    >(
        &self,
        worker: &W,
        random_seed: RandomSeed,
        current_active_ms: i64,
    ) -> Result<WaitOutcome, Error> {
        let person_uuid = self.person_uuid.clone().ok_or(Error::MissingPersonUuid)?;
        let started_at = self.started_at.ok_or(Error::MissingStartedAt)?;

        let elapsed = current_active_ms.saturating_sub(self.start_active_ms);
        if elapsed >= self.duration_ms {
            let get_args = crate::capability::event::GetArgs::new()
                .with_person_uuid(person_uuid.clone());
            let events = worker
                .get_events(get_args)
                .await
                .map_err(Error::FailedToGetEvents)?;
            let has_recent_events = events.iter().any(|event| event.timestamp >= started_at);

            if has_recent_events {
                return Ok(WaitOutcome::Ready);
            }

            let persons_name: PersonName = worker
                .get_persons_name(person_uuid.clone())
                .await
                .map_err(Error::FailedToGetPersonsName)?;

            let scene_uuid = worker
                .get_persons_current_scene_uuid(&person_uuid)
                .await
                .map_err(|err| Error::CouldNotGetPersonsScene {
                    person_uuid: person_uuid.clone(),
                    details: err,
                })?;

            let message_type_args = match scene_uuid.clone() {
                Some(scene_uuid) => MessageTypeArgs::SceneByUuid { scene_uuid },
                None => MessageTypeArgs::Direct {
                    from: MessageSender::RealWorldUser,
                },
            };

            let maybe_state_of_mind: Option<StateOfMind> = worker
                .get_latest_state_of_mind(&person_uuid)
                .await
                .map_err(Error::FailedToGetStateOfMind)?;

            let state_of_mind: StateOfMind = match maybe_state_of_mind {
                Some(som) => som,
                None => Err(Error::NoStateOfMindFound {
                    person_uuid: person_uuid.clone(),
                })?,
            };

            let events_text = events
                .iter()
                .map(|event| event.to_text())
                .collect::<Vec<String>>();

            let minutes_waiting = (self.duration_ms / 60000).max(0);
            let situation = format!(
                "{} decided to wait {} minutes, and nothing happened.",
                persons_name.to_string(),
                minutes_waiting
            );

            let memories_prompt = worker
                .create_memory_query_prompt(
                    persons_name,
                    message_type_args,
                    events_text,
                    &state_of_mind.content,
                    &situation,
                )
                .await
                .map_err(Error::CouldNotCreateMemoriesPrompt)?;

            let memories: Vec<Memory> = worker
                .search_memories(memories_prompt.prompt, 8)
                .await
                .map_err(Error::FailedToSearchMemories)?
                .into_iter()
                .map(|memory_search_result: MemorySearchResult| Memory::from(memory_search_result))
                .collect();

            let maybe_person_identity: Option<String> = worker
                .get_person_identity(&person_uuid)
                .await
                .map_err(Error::FailedToGetPersonIdentity)?;

            let person_identity: String = match maybe_person_identity {
                Some(identity) => identity,
                None => Err(Error::NoPersonIdentityFound {
                    person_uuid: person_uuid.clone(),
                })?,
            };

            let actions = worker
                .get_reaction(memories, person_identity, state_of_mind.content, situation)
                .await
                .map_err(Error::GetPersonReactionCompletionError)?;

            for action in actions {
                match action {
                    PersonAction::Wait { duration } => {
                        let duration_i64: i64 = duration.min(i64::MAX as u64) as i64;
                        let person_waiting_job = PersonWaitingJob::new(
                            person_uuid.clone(),
                            duration_i64,
                            current_active_ms,
                        );
                        let wait_job = JobKind::PersonWaiting(person_waiting_job);
                        worker.unshift_job(wait_job).await.map_err(|err| {
                            Error::PersonCouldNotWait {
                                person_uuid: person_uuid.clone(),
                                error: err,
                            }
                        })?;
                    }
                    PersonAction::SayInScene { comment } => {
                        let sender = MessageSender::AiPerson(person_uuid.clone());

                        let scene_uuid = worker
                            .get_persons_current_scene_uuid(&person_uuid)
                            .await
                            .map_err(|err| Error::CouldNotGetPersonsScene {
                                person_uuid: person_uuid.clone(),
                                details: err,
                            })?
                            .ok_or(Error::CouldNotGetPersonsScene {
                                person_uuid: person_uuid.clone(),
                                details: "Person is not in any scene".to_string(),
                            })?;

                        let send_message_to_scene_job = SendMessageToSceneJob {
                            sender,
                            scene_uuid: scene_uuid.clone(),
                            content: comment,
                            random_seed: random_seed.clone(),
                        };
                        let job_kind = JobKind::SendMessageToScene(send_message_to_scene_job);
                        worker.unshift_job(job_kind).await.map_err(|err| {
                            Error::PersonCouldNotSayInScene {
                                scene_uuid: scene_uuid,
                                details: err,
                                subject: person_uuid.to_uuid().to_string(),
                            }
                        })?;
                    }
                }
            }

            Ok(WaitOutcome::Ready)
        } else {
            Ok(WaitOutcome::NotReady)
        }
    }
}
