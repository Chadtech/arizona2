use crate::domain::memory::Memory;
use crate::domain::person_task::PersonTask;
use crate::domain::person_task::PersonTaskOutcomeCheck;
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
    async fn summarize_reaction_events(&self, events_text: String) -> Result<String, String>;
    async fn preview_reaction_prompts(
        &self,
        memories: Vec<Memory>,
        person_uuid: PersonUuid,
        situation: String,
    ) -> Result<ReactionPromptPreview, String>;
    async fn get_reaction(
        &self,
        memories: Vec<Memory>,
        person_uuid: PersonUuid,
        state_of_mind: String,
        situation: String,
    ) -> Result<PersonReaction, String>;

    async fn classify_current_task_outcome(
        &self,
        task: PersonTask,
        situation: String,
        action_summary: Option<String>,
    ) -> Result<PersonTaskOutcomeCheck, String>;

    async fn infer_updated_task_state(
        &self,
        task: PersonTask,
        situation: String,
        action_summary: Option<String>,
    ) -> Result<String, String>;
}
