use crate::capability::reaction::ReactionCapability;
use crate::domain::memory::Memory;
use crate::open_ai::completion::CompletionError;
use crate::person_actions::PersonAction;
use crate::worker::Worker;

impl ReactionCapability for Worker {
    async fn get_reaction(
        memories: Vec<Memory>,
        person_identity: String,
        situation: String,
        state_of_mind: String,
    ) -> Result<Vec<PersonAction>, CompletionError> {
        todo!()
    }
}
