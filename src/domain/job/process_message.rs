use crate::capability::event::EventCapability;
use crate::capability::job::JobCapability;
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
use crate::domain::job::person_action_handler::ActionHandleError;
use crate::domain::job::process_reaction_common::{self, SceneReactionTrigger};
use crate::domain::message_uuid::MessageUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::random_seed::RandomSeed;
use crate::domain::scene_uuid::SceneUuid;
use crate::nice_display::NiceDisplay;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMessageJob {
    pub message_uuid: MessageUuid,
    pub recipient_person_uuid: PersonUuid,
}

pub enum Error {
    FailedToGetMessage(String),
    MessageNotFound,
    GetPersonReaction(String),
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
    FailedToGetSendersName {
        person_uuid: PersonUuid,
        details: String,
    },
    FailedToGetPersonsName(String),
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
    FailedToGetHibernationState {
        person_uuid: PersonUuid,
        details: String,
    },
    FailedToGetEnabledState {
        person_uuid: PersonUuid,
        details: String,
    },
    FailedToCreateMemory(String),
    FailedToCreateReflectionStateOfMind(String),
    FailedToCreateReflectionMemory(String),
    FailedToCreateReflectionMotivation(String),
    FailedToDeleteReflectionMotivation(String),
    Action(ActionHandleError),
    Reflection(String),
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::FailedToGetMessage(details) => {
                format!("Failed to get message: {}", details)
            }
            Error::MessageNotFound => "Message not found".to_string(),
            Error::GetPersonReaction(err) => {
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
            Error::FailedToGetHibernationState {
                person_uuid,
                details,
            } => {
                format!(
                    "Failed to get hibernation state for {}: {}",
                    person_uuid.to_uuid(),
                    details
                )
            }
            Error::FailedToGetEnabledState {
                person_uuid,
                details,
            } => {
                format!(
                    "Failed to get enabled state for {}: {}",
                    person_uuid.to_uuid(),
                    details
                )
            }
            Error::FailedToCreateMemory(err) => {
                format!("Failed to create memory:\n{}", err)
            }
            Error::FailedToCreateReflectionStateOfMind(err) => {
                format!("Failed to create reflection state of mind:\n{}", err)
            }
            Error::FailedToCreateReflectionMemory(err) => {
                format!("Failed to create reflection memory:\n{}", err)
            }
            Error::FailedToCreateReflectionMotivation(err) => {
                format!("Failed to create reflection motivation:\n{}", err)
            }
            Error::FailedToDeleteReflectionMotivation(err) => {
                format!("Failed to delete reflection motivation:\n{}", err)
            }
            Error::Action(err) => err.to_nice_error().to_string(),
            Error::Reflection(err) => {
                format!("Reflection error:\n{}", err)
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
            + ReactionHistoryCapability
            + ReflectionCapability
            + LogCapability
            + LogEventCapability
            + MotivationCapability
            + JobCapability
            + Sync,
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

        process_reaction_common::run_scene_reaction(
            worker,
            &self.recipient_person_uuid,
            &message.scene_uuid,
            SceneReactionTrigger::NewMessages,
            random_seed,
            current_active_ms,
        )
        .await
    }
}
