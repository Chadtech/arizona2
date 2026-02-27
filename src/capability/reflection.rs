use crate::domain::memory::Memory;
use crate::domain::motivation_uuid::MotivationUuid;
use crate::domain::person_uuid::PersonUuid;

pub enum ReflectionChange {
    StateOfMind { content: String },
    MemorySummary { summary: String },
    NewMotivation { content: String, priority: i64 },
    DeleteMotivation { motivation_uuid: MotivationUuid },
}

pub trait ReflectionCapability {
    async fn get_reflection_changes(
        &self,
        memories: Vec<Memory>,
        person_uuid: PersonUuid,
        person_identity: String,
        state_of_mind: String,
        situation: String,
    ) -> Result<Vec<ReflectionChange>, String>;
}
