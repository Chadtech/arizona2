#[derive(Debug, Clone)]
pub struct MessageUuid(uuid::Uuid);

impl MessageUuid {
    pub fn new() -> Self {
        MessageUuid(uuid::Uuid::now_v7())
    }
    pub fn to_uuid(&self) -> uuid::Uuid {
        self.0
    }
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        MessageUuid(uuid)
    }
}

impl From<uuid::Uuid> for MessageUuid {
    fn from(value: uuid::Uuid) -> Self {
        MessageUuid(value)
    }
}
