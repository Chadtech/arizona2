use super::scene::SceneCapability;
use crate::domain::memory_uuid::MemoryUuid;
use crate::domain::message::MessageSender;
use crate::domain::person_name::PersonName;
use crate::domain::person_uuid::PersonUuid;
use crate::domain::scene_uuid::SceneUuid;

pub struct NewMemory {
    pub memory_uuid: MemoryUuid,
    pub content: String,
    pub person_uuid: PersonUuid,
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

pub enum MessageTypeArgs {
    Scene {
        scene_name: String,
        scene_description: String,
        people: Vec<String>,
    },
    SceneByUuid {
        scene_uuid: SceneUuid,
    },
    Direct {
        from: MessageSender,
    },
}

pub trait MemoryCapability: SceneCapability {
    async fn create_memory(&self, new_memory: NewMemory) -> Result<MemoryUuid, String>;
    async fn maybe_create_memories_from_description(
        &self,
        person_uuid: PersonUuid,
        description: String,
    ) -> Result<Vec<MemoryUuid>, String>;
    async fn create_memory_query_prompt(
        &self,
        person_recalling: PersonName,
        message_type_args: MessageTypeArgs,
        recent_events: Vec<String>,
        state_of_mind: &String,
        situation: &String,
    ) -> Result<MemoryQueryPrompt, String>;
    async fn search_memories(
        &self,
        query: String,
        limit: i64,
    ) -> Result<Vec<MemorySearchResult>, String>;
}
