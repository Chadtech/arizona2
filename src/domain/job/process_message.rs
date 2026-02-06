use super::send_message_to_scene::SendMessageToSceneJob;
use crate::capability;
use crate::capability::event::EventCapability;
use crate::capability::job::JobCapability;
use crate::capability::memory::{MemoryCapability, MemorySearchResult, MessageTypeArgs};
use crate::capability::person::PersonCapability;
use crate::capability::person_identity::PersonIdentityCapability;
use crate::capability::reaction::ReactionCapability;
use crate::capability::state_of_mind::StateOfMindCapability;
use crate::domain::job::person_waiting::PersonWaitingJob;
use crate::domain::job::JobKind;
use crate::domain::memory::Memory;
use crate::domain::message::{Message, MessageRecipient, MessageSender};
use crate::domain::person_name::PersonName;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::random_seed::RandomSeed;
use crate::domain::scene_uuid::SceneUuid;
use crate::domain::state_of_mind::StateOfMind;
use crate::open_ai::completion::CompletionError;
use crate::person_actions::PersonAction;
use crate::{
    capability::{message::MessageCapability, scene::SceneCapability},
    domain::message_uuid::MessageUuid,
    nice_display::NiceDisplay,
};
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMessageJob {
    pub message_uuid: MessageUuid,
    #[serde(default)]
    pub recipient_person_uuid: Option<PersonUuid>,
}

pub enum Error {
    FailedToGetMessage(String),
    MessageNotFound,
    FailedToMarkMessageRead(String),
    GetPersonReactionError(String),
    FailedToGetEvents(String),
    FailedToGetStateOfMind(String),
    NoStateOfMindFound {
        person_uuid: PersonUuid,
    },
    CouldNotCreateMemoriesPrompt(String),
    FailedToSearchMemories(String),
    FailedToGetPersonIdentity(String),
    NoPersonIdentityFound {
        person_uuid: PersonUuid,
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
    CouldNotGetPersonsScene {
        person_uuid: PersonUuid,
        details: String,
    },
    FailedToGetSendersName {
        person_uuid: PersonUuid,
        details: String,
    },
    FailedToGetPersonsName(String),
    FailedToGetSceneName {
        scene_uuid: SceneUuid,
        details: String,
    },
    SceneNameNotFound {
        scene_uuid: SceneUuid,
    },
    FailedToGetSceneDescription {
        scene_uuid: SceneUuid,
        details: String,
    },
    SceneDescriptionNotFound {
        scene_uuid: SceneUuid,
    },
    FailedToGetSceneParticipants {
        scene_uuid: SceneUuid,
        details: String,
    },
    FailedToCreateMemory(String),
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::FailedToGetMessage(details) => {
                format!("Failed to get message: {}", details)
            }
            Error::MessageNotFound => "Message not found".to_string(),
            Error::FailedToMarkMessageRead(details) => {
                format!("Failed to mark message as read: {}", details)
            }
            Error::GetPersonReactionError(err) => {
                format!("Failed to get person reaction: {}", err)
            }
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
            Error::CouldNotCreateMemoriesPrompt(err) => {
                format!("Could not create memories prompt: {}", err)
            }
            Error::FailedToSearchMemories(err) => {
                format!("Failed to search memories: {}", err)
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
            Error::FailedToGetSendersName {
                person_uuid,
                details,
            } => {
                format!(
                    "Failed to get person's name for {}: {}",
                    person_uuid.to_uuid(),
                    details
                )
            }
            Error::FailedToGetPersonsName(err) => {
                format!("Failed to get person's name: {}", err)
            }
            Error::FailedToGetSceneName {
                scene_uuid,
                details,
            } => {
                format!(
                    "Failed to get scene name for {}: {}",
                    scene_uuid.to_uuid(),
                    details
                )
            }
            Error::SceneNameNotFound { scene_uuid } => {
                format!("Scene name not found for {}", scene_uuid.to_uuid())
            }
            Error::FailedToGetSceneDescription {
                scene_uuid,
                details,
            } => {
                format!(
                    "Failed to get scene description for {}: {}",
                    scene_uuid.to_uuid(),
                    details
                )
            }
            Error::SceneDescriptionNotFound { scene_uuid } => {
                format!("Scene description not found for {}", scene_uuid.to_uuid())
            }
            Error::FailedToGetSceneParticipants {
                scene_uuid,
                details,
            } => {
                format!(
                    "Failed to get scene participants for {}: {}",
                    scene_uuid.to_uuid(),
                    details
                )
            }
            Error::FailedToCreateMemory(err) => {
                format!("Failed to create memory:\n{}", err)
            }
        }
    }
}

impl ProcessMessageJob {
    pub async fn run<
        W: MessageCapability
            + SceneCapability
            + ReactionCapability
            + MemoryCapability
            + PersonCapability
            + EventCapability
            + StateOfMindCapability
            + PersonIdentityCapability
            + JobCapability,
    >(
        self,
        worker: &W,
        random_seed: RandomSeed,
        current_active_ms: i64,
    ) -> Result<(), Error> {
        let maybe_message = worker
            .get_message_by_uuid(&self.message_uuid)
            .await
            .map_err(Error::FailedToGetMessage)?;

        let message = match maybe_message {
            Some(msg) => msg,
            None => Err(Error::MessageNotFound)?,
        };

        let person_uuid = if let Some(ref person_uuid) = self.recipient_person_uuid {
            Some(person_uuid)
        } else if let Some(MessageRecipient::Person(ref person_uuid)) = message.recipient {
            Some(person_uuid)
        } else {
            None
        };

        match person_uuid {
            Some(person_uuid) => {
                let action = process_message_for_person(worker, &message, &person_uuid).await?;

                match &action {
                    PersonAction::Wait { duration } => {
                        // Cap at i64::MAX if u64 exceeds it
                        let duration_i64: i64 = (*duration).min(i64::MAX as u64) as i64;
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
                            content: comment.clone(),
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

                let situation = build_situation(worker, &message).await?;
                let action_summary = summarize_action(action);
                let description = if action_summary.is_empty() {
                    situation
                } else {
                    format!("{}\n\nResponse:\n{}", situation, action_summary)
                };

                worker
                    .maybe_create_memories_from_description(person_uuid.clone(), description)
                    .await
                    .map_err(Error::FailedToCreateMemory)?;
            }
            None => {
                //
            }
        }

        worker
            .mark_message_read(&self.message_uuid)
            .await
            .map_err(Error::FailedToMarkMessageRead)?;

        // Placeholder for additional processing logic on the message

        Ok(())
    }
}

fn summarize_action(action: PersonAction) -> String {
    match action {
        PersonAction::Wait { duration } => {
            format!("Waited for {} seconds.", duration)
        }
        PersonAction::SayInScene { comment } => {
            format!("Spoke in scene: {}", comment)
        }
    }
}

async fn build_situation<W: SceneCapability + PersonCapability>(
    worker: &W,
    message: &Message,
) -> Result<String, Error> {
    let sender_name = match &message.sender {
        MessageSender::AiPerson(sender_person_uuid) => worker
            .get_persons_name(sender_person_uuid.clone())
            .await
            .map_err(|err| Error::FailedToGetSendersName {
                person_uuid: sender_person_uuid.clone(),
                details: err,
            })?,
        MessageSender::RealWorldUser => PersonName::from_string("Chadtech".to_string()),
    };

    match &message.scene_uuid {
        Some(scene_uuid) => {
            let scene_name = worker
                .get_scene_name(scene_uuid)
                .await
                .map_err(|err| Error::FailedToGetSceneName {
                    scene_uuid: scene_uuid.clone(),
                    details: err,
                })?
                .ok_or_else(|| Error::SceneNameNotFound {
                    scene_uuid: scene_uuid.clone(),
                })?;

            let scene_description = worker
                .get_scene_description(scene_uuid)
                .await
                .map_err(|err| Error::FailedToGetSceneDescription {
                    scene_uuid: scene_uuid.clone(),
                    details: err,
                })?
                .ok_or_else(|| Error::SceneDescriptionNotFound {
                    scene_uuid: scene_uuid.clone(),
                })?;

            let participants = worker
                .get_scene_current_participants(scene_uuid)
                .await
                .map_err(|err| Error::FailedToGetSceneParticipants {
                    scene_uuid: scene_uuid.clone(),
                    details: err,
                })?;

            let participant_names = participants
                .iter()
                .map(|participant| participant.person_name.to_string())
                .collect::<Vec<String>>();

            let participant_list = if participant_names.is_empty() {
                "none".to_string()
            } else {
                participant_names.join(", ")
            };

            Ok(format!(
                "You are in the scene \"{}\". {}\n\nOther people present: {}\n\n{} said:\n\n{}",
                scene_name,
                scene_description,
                participant_list,
                sender_name.as_str(),
                message.content
            ))
        }
        None => Ok(format!(
            "You received a direct message from {}:\n\n{}",
            sender_name.as_str(),
            message.content
        )),
    }
}

async fn process_message_for_person<
    'a,
    W: MessageCapability
        + SceneCapability
        + ReactionCapability
        + MemoryCapability
        + PersonCapability
        + EventCapability
        + StateOfMindCapability
        + PersonIdentityCapability,
>(
    worker: &W,
    message: &'a Message,
    person_uuid: &'a PersonUuid,
) -> Result<PersonAction, Error> {
    let persons_name: PersonName = worker
        .get_persons_name(person_uuid.clone())
        .await
        .map_err(Error::FailedToGetPersonsName)?;

    let message_type_args = match &message.scene_uuid {
        None => MessageTypeArgs::Direct {
            from: message.sender.clone(),
        },
        Some(scene_uuid) => MessageTypeArgs::SceneByUuid {
            scene_uuid: scene_uuid.clone(),
        },
    };

    let get_args: capability::event::GetArgs =
        capability::event::GetArgs::new().with_person_uuid(person_uuid.clone());

    let events = worker
        .get_events(get_args)
        .await
        .map_err(Error::FailedToGetEvents)?
        .iter()
        .map(|event| event.to_text())
        .collect::<Vec<String>>();

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

    let situation = build_situation(worker, message).await?;

    let memories_prompt = worker
        .create_memory_query_prompt(
            persons_name,
            message_type_args,
            events,
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
        .get_person_identity(person_uuid)
        .await
        .map_err(Error::FailedToGetPersonIdentity)?;

    let person_identity: String = match maybe_person_identity {
        Some(identity) => identity,
        None => Err(Error::NoPersonIdentityFound {
            person_uuid: person_uuid.clone(),
        })?,
    };

    worker
        .get_reaction(memories, person_identity, state_of_mind.content, situation)
        .await
        .map_err(Error::GetPersonReactionError)
}
