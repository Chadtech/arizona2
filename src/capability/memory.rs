use crate::domain::memory_uuid::MemoryUuid;
use crate::domain::person_name::PersonName;

pub struct NewMemory {
    pub memory_uuid: MemoryUuid,
    pub content: String,
    pub person_name: PersonName,
}

pub struct MemoryQueryPrompt {
    pub prompt: String,
}

pub trait MemoryCapability {
    async fn create_memory(&self, new_memory: NewMemory) -> Result<MemoryUuid, String>;
    async fn create_memory_query_prompt(
        &self,
        person_recalling: String,
        people: Vec<String>,
        scene_name: String,
        scene_description: String,
        recent_events: Vec<String>,
        state_of_mind: String,
    ) -> Result<MemoryQueryPrompt, String>;
}
