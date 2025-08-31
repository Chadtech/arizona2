use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct StateOfMindUuid(Uuid);

impl StateOfMindUuid {
    pub fn new() -> Self {
        StateOfMindUuid(Uuid::now_v7())
    }
    pub fn to_uuid(&self) -> Uuid {
        self.0
    }
    pub fn from_uuid(uuid: Uuid) -> Self {
        StateOfMindUuid(uuid)
    }
}
