use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MemoryUuid(Uuid);

impl MemoryUuid {
    pub fn to_uuid(&self) -> Uuid {
        self.0
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }

    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(uuid: Uuid) -> MemoryUuid {
        MemoryUuid(uuid)
    }
}
