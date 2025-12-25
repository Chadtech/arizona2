use crate::domain::memory::Memory;
use crate::open_ai::completion::CompletionError;
use crate::person_actions::PersonAction;

pub trait ReactionCapability {
    async fn get_reaction(
        memories: Vec<Memory>,
        person_identity: String,
        situation: String,
        state_of_mind: String,
    ) -> Result<Vec<PersonAction>, CompletionError>;
}
