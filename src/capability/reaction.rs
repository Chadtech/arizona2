use crate::domain::memory::Memory;
use crate::person_actions::PersonAction;

pub trait ReactionCapability {
    async fn get_reaction(
        &self,
        memories: Vec<Memory>,
        person_identity: String,
        state_of_mind: String,
        situation: String,
    ) -> Result<PersonAction, String>;
}
