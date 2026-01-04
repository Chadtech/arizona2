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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMessageJob {
    pub message_uuid: MessageUuid,
}

pub enum Error {
    FailedToGetMessage(String),
    MessageNotFound,
    FailedToMarkMessageRead(String),
    GetPersonReactionCompletionError(CompletionError),
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
            Error::GetPersonReactionCompletionError(err) => {
                format!("Failed to get person reaction: {}", err.message())
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
    ) -> Result<(), Error> {
        let maybe_message = worker
            .get_message_by_uuid(&self.message_uuid)
            .await
            .map_err(Error::FailedToGetMessage)?;

        let message = match maybe_message {
            Some(msg) => msg,
            None => return Err(Error::MessageNotFound),
        };

        match message.recipient {
            MessageRecipient::Person(ref person_uuid) => {
                let actions = process_message_for_person(worker, &message, person_uuid).await?;

                for action in actions {
                    match action {
                        PersonAction::Wait { duration } => {
                            // Cap at i64::MAX if u64 exceeds it
                            let duration_i64: i64 = duration.min(i64::MAX as u64) as i64;
                            let person_waiting_job = PersonWaitingJob::new(duration_i64);
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
            }
            MessageRecipient::RealWorldUser => {
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
) -> Result<Vec<PersonAction>, Error> {
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

    let situation = format!("{} said:\n\n{}", sender_name.to_string(), message.content);

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

    let actions = worker
        .get_reaction(memories, person_identity, state_of_mind.content, situation)
        .await
        .map_err(Error::GetPersonReactionCompletionError)?;

    Ok(actions)
}
