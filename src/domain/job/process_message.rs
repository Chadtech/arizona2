use crate::capability::event::EventCapability;
use crate::capability::job::JobCapability;
use crate::capability::log_event::LogEventCapability;
use crate::capability::logging::LogCapability;
use crate::capability::memory::MemoryCapability;
use crate::capability::message::MessageCapability;
use crate::capability::motivation::MotivationCapability;
use crate::capability::person::PersonCapability;
use crate::capability::person_identity::PersonIdentityCapability;
use crate::capability::person_task::PersonTaskCapability;
use crate::capability::reaction::ReactionCapability;
use crate::capability::reaction_history::ReactionHistoryCapability;
use crate::capability::reflection::ReflectionCapability;
use crate::capability::scene::SceneCapability;
use crate::capability::state_of_mind::StateOfMindCapability;
use crate::domain::job::process_reaction_common::{self, SceneReactionTrigger};
use crate::domain::message_uuid::MessageUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::random_seed::RandomSeed;
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
    Reaction(process_reaction_common::Error),
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::FailedToGetMessage(details) => {
                format!("Failed to get message: {}", details)
            }
            Error::MessageNotFound => "Message not found".to_string(),
            Error::Reaction(err) => err.message(),
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
            + PersonTaskCapability
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
        .map_err(Error::Reaction)
    }
}
