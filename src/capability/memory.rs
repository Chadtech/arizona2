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

#[derive(Clone, Debug)]
pub struct MemorySearchResult {
    pub memory_uuid: MemoryUuid,
    pub content: String,
    pub distance: f64,
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
    async fn search_memories(
        &self,
        query: String,
        limit: i64,
    ) -> Result<Vec<MemorySearchResult>, String>;
}
