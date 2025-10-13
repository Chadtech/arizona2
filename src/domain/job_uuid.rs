use uuid::Uuid;
#[derive(Debug, Clone)]
pub struct JobUuid(Uuid);

impl JobUuid {
    pub fn new() -> Self {
        JobUuid(Uuid::now_v7())
    }
    pub fn to_uuid(&self) -> Uuid {
        self.0
    }
    pub fn from_uuid(uuid: Uuid) -> Self {
        JobUuid(uuid)
    }
}
