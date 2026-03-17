use crate::domain::memory::Memory;
use crate::domain::person_uuid::PersonUuid;
use crate::person_actions::PersonReaction;

#[derive(Debug, Clone)]
pub struct ReactionPromptPreview {
    pub thinking_system_prompt: String,
    pub thinking_user_prompt: String,
    pub action_system_prompt: String,
    pub action_user_prompt: String,
}

pub trait ReactionCapability {
    async fn preview_reaction_prompts(
        &self,
        memories: Vec<Memory>,
        person_uuid: PersonUuid,
        person_identity: String,
        state_of_mind: String,
        situation: String,
    ) -> Result<ReactionPromptPreview, String>;
    async fn get_reaction(
        &self,
        memories: Vec<Memory>,
        person_uuid: PersonUuid,
        person_identity: String,
        state_of_mind: String,
        situation: String,
    ) -> Result<PersonReaction, String>;
}
