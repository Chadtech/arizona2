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
use crate::domain::person_uuid::PersonUuid;
use crate::domain::random_seed::RandomSeed;
use crate::domain::scene_uuid::SceneUuid;
use crate::nice_display::NiceDisplay;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessPersonJoinJob {
    pub scene_uuid: SceneUuid,
    pub joined_person_uuid: PersonUuid,
    pub recipient_person_uuid: PersonUuid,
}

pub enum Error {
    Reaction(process_reaction_common::Error),
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::Reaction(err) => err.message(),
        }
    }
}

impl ProcessPersonJoinJob {
    pub async fn run<
        W: SceneCapability
            + ReactionCapability
            + MemoryCapability
            + MessageCapability
            + PersonCapability
            + EventCapability
            + StateOfMindCapability
            + PersonIdentityCapability
            + PersonTaskCapability
            + ReflectionCapability
            + LogCapability
            + LogEventCapability
            + MotivationCapability
            + ReactionHistoryCapability
            + JobCapability
            + Sync,
    >(
        self,
        worker: &W,
        random_seed: RandomSeed,
        current_active_ms: i64,
    ) -> Result<(), Error> {
        process_reaction_common::run_scene_reaction(
            worker,
            &self.recipient_person_uuid,
            &self.scene_uuid,
            SceneReactionTrigger::PersonJoined {
                joined_person_uuid: self.joined_person_uuid,
            },
            random_seed,
            current_active_ms,
        )
        .await
        .map_err(Error::Reaction)
    }
}
