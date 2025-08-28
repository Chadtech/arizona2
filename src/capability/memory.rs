use crate::domain::memory_uuid::MemoryUuid;
use crate::domain::person_name::PersonName;

pub struct NewMemory {
    pub memory_uuid: MemoryUuid,
    pub content: String,
    pub person_name: PersonName,
}

pub trait MemoryCapability {
    async fn create_memory(&self, new_memory: NewMemory) -> Result<MemoryUuid, String>;
}
