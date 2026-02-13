use crate::capability;
use crate::capability::event::EventCapability;
use crate::capability::job::JobCapability;
use crate::capability::memory::{MemoryCapability, MemorySearchResult, MessageTypeArgs};
use crate::capability::person::PersonCapability;
use crate::capability::person_identity::PersonIdentityCapability;
use crate::capability::reaction::ReactionCapability;
use crate::capability::state_of_mind::StateOfMindCapability;
use crate::domain::job::person_waiting::PersonWaitingJob;
use crate::domain::job::send_message_to_scene::send_scene_message_and_enqueue_recipients;
use crate::domain::job::JobKind;
use crate::domain::memory::Memory;
use crate::domain::message::{Message, MessageRecipient, MessageSender};
use crate::domain::actor_uuid::ActorUuid;
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
    FailedToSendSceneMessage {
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
    SceneMessageRecipientMissing {
        message_uuid: MessageUuid,
    },
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
    FailedToGetUnhandledSceneMessages {
        scene_uuid: SceneUuid,
        details: String,
    },
    FailedToMarkSceneMessagesHandled {
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
            Error::FailedToSendSceneMessage {
                scene_uuid,
                details,
                subject,
            } => {
                format!(
                    "Person {} could not send message in scene {}: {}",
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
            Error::SceneMessageRecipientMissing { message_uuid } => {
                format!(
                    "Scene message {} is missing a recipient",
                    message_uuid.to_uuid()
                )
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
            Error::FailedToGetUnhandledSceneMessages {
                scene_uuid,
                details,
            } => {
                format!(
                    "Failed to get unhandled scene messages for {}: {}",
                    scene_uuid.to_uuid(),
                    details
                )
            }
            Error::FailedToMarkSceneMessagesHandled {
                scene_uuid,
                details,
            } => {
                format!(
                    "Failed to mark scene messages handled for {}: {}",
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

        match (person_uuid, &message.scene_uuid) {
            (Some(person_uuid), Some(scene_uuid)) => {
                let pending_messages = worker
                    .get_unhandled_scene_messages_for_person(person_uuid, scene_uuid)
                    .await
                    .map_err(|err| Error::FailedToGetUnhandledSceneMessages {
                        scene_uuid: scene_uuid.clone(),
                        details: err,
                    })?;

                if pending_messages.is_empty() {
                    return Ok(());
                }

                let situation = build_scene_situation(
                    worker,
                    scene_uuid,
                    &pending_messages,
                    person_uuid,
                )
                .await?;

                let action = process_message_for_person(
                    worker,
                    MessageTypeArgs::SceneByUuid {
                        scene_uuid: scene_uuid.clone(),
                    },
                    &situation,
                    person_uuid,
                )
                .await?;

                handle_person_action(
                    worker,
                    &action,
                    person_uuid,
                    random_seed.clone(),
                    current_active_ms,
                )
                .await?;

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

                let handled_ids = pending_messages
                    .into_iter()
                    .map(|msg| msg.uuid)
                    .collect::<Vec<_>>();

                worker
                    .mark_scene_messages_handled_for_person(person_uuid, handled_ids)
                    .await
                    .map_err(|err| Error::FailedToMarkSceneMessagesHandled {
                        scene_uuid: scene_uuid.clone(),
                        details: err,
                    })?;
            }
            (Some(person_uuid), None) => {
                let situation = build_direct_situation(worker, &message).await?;

                let action = process_message_for_person(
                    worker,
                    MessageTypeArgs::Direct {
                        from: message.sender.clone(),
                    },
                    &situation,
                    person_uuid,
                )
                .await?;

                handle_person_action(
                    worker,
                    &action,
                    person_uuid,
                    random_seed.clone(),
                    current_active_ms,
                )
                .await?;

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

                worker
                    .mark_message_read(&self.message_uuid)
                    .await
                    .map_err(Error::FailedToMarkMessageRead)?;
            }
            (None, Some(_)) => {
                return Err(Error::SceneMessageRecipientMissing {
                    message_uuid: self.message_uuid.clone(),
                });
            }
            (None, None) => {
                //
            }
        }

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

async fn handle_person_action<
    W: SceneCapability + JobCapability + PersonCapability + MessageCapability,
>(
    worker: &W,
    action: &PersonAction,
    person_uuid: &PersonUuid,
    random_seed: RandomSeed,
    current_active_ms: i64,
) -> Result<(), Error> {
    match action {
        PersonAction::Wait { duration } => {
            // Cap at i64::MAX if u64 exceeds it
            let duration_i64: i64 = (*duration).min(i64::MAX as u64) as i64;
            let person_waiting_job =
                PersonWaitingJob::new(person_uuid.clone(), duration_i64, current_active_ms);
            let wait_job = JobKind::PersonWaiting(person_waiting_job);
            worker
                .unshift_job(wait_job)
                .await
                .map_err(|err| Error::PersonCouldNotWait {
                    person_uuid: person_uuid.clone(),
                    error: err,
                })?;
        }
        PersonAction::SayInScene { comment } => {
            let sender = MessageSender::AiPerson(person_uuid.clone());

            let scene_uuid = worker
                .get_persons_current_scene_uuid(person_uuid)
                .await
                .map_err(|err| Error::CouldNotGetPersonsScene {
                    person_uuid: person_uuid.clone(),
                    details: err,
                })?
                .ok_or(Error::CouldNotGetPersonsScene {
                    person_uuid: person_uuid.clone(),
                    details: "Person is not in any scene".to_string(),
                })?;

            send_scene_message_and_enqueue_recipients(
                worker,
                sender,
                scene_uuid.clone(),
                comment.clone(),
                random_seed,
            )
            .await
            .map_err(|err| Error::FailedToSendSceneMessage {
                scene_uuid: scene_uuid.clone(),
                details: err.to_nice_error().to_string(),
                subject: person_uuid.to_uuid().to_string(),
            })?;
        }
    }

    Ok(())
}

async fn build_direct_situation<W: PersonCapability>(
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

    Ok(format!(
        "You received a direct message from {}:\n\n{}",
        sender_name.as_str(),
        message.content
    ))
}

async fn build_scene_situation<W: SceneCapability + PersonCapability>(
    worker: &W,
    scene_uuid: &SceneUuid,
    messages: &[Message],
    person_uuid: &PersonUuid,
) -> Result<String, Error> {
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
        .filter(|participant| match &participant.actor_uuid {
            ActorUuid::AiPerson(uuid) => uuid.to_uuid() != person_uuid.to_uuid(),
            ActorUuid::RealWorldUser => true,
        })
        .map(|participant| participant.person_name.to_string())
        .collect::<Vec<String>>();

    let participant_list = if participant_names.is_empty() {
        "none".to_string()
    } else {
        participant_names.join(", ")
    };

    let mut lines = Vec::new();
    for message in messages {
        let sender_label = match &message.sender {
            MessageSender::AiPerson(sender_person_uuid) => worker
                .get_persons_name(sender_person_uuid.clone())
                .await
                .map_err(|err| Error::FailedToGetSendersName {
                    person_uuid: sender_person_uuid.clone(),
                    details: err,
                })?
                .to_string(),
            MessageSender::RealWorldUser => "Chadtech".to_string(),
        };

        lines.push(format!("{}: {}", sender_label, message.content));
    }

    let messages_block = if lines.is_empty() {
        "No new messages.".to_string()
    } else {
        lines.join("\n")
    };

    Ok(format!(
        "You are in the scene \"{}\". {}\n\nOther people present: {}\n\nMessages received (oldest to newest):\n{}",
        scene_name,
        scene_description,
        participant_list,
        messages_block
    ))
}

async fn process_message_for_person<
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
    message_type_args: MessageTypeArgs,
    situation: &String,
    person_uuid: &PersonUuid,
) -> Result<PersonAction, Error> {
    let persons_name: PersonName = worker
        .get_persons_name(person_uuid.clone())
        .await
        .map_err(Error::FailedToGetPersonsName)?;

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

    let memories_prompt = worker
        .create_memory_query_prompt(
            persons_name,
            message_type_args,
            events,
            &state_of_mind.content,
            situation,
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
        .get_reaction(
            memories,
            person_identity,
            state_of_mind.content,
            situation.to_string(),
        )
        .await
        .map_err(Error::GetPersonReactionError)
}
