use crate::domain::memory::Memory;
use crate::domain::person_uuid::PersonUuid;
use crate::person_actions::PersonReaction;

pub trait ReactionCapability {
    async fn get_reaction(
        &self,
        memories: Vec<Memory>,
        person_uuid: PersonUuid,
        person_identity: String,
        state_of_mind: String,
        situation: String,
    ) -> Result<PersonReaction, String>;
}
