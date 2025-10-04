#[derive(Debug, Clone)]
pub struct SceneParticipantUuid(uuid::Uuid);

impl SceneParticipantUuid {
    pub fn new() -> Self {
        SceneParticipantUuid(uuid::Uuid::now_v7())
    }
    pub fn to_uuid(&self) -> uuid::Uuid {
        self.0
    }
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        SceneParticipantUuid(uuid)
    }
}

impl From<uuid::Uuid> for SceneParticipantUuid {
    fn from(value: uuid::Uuid) -> Self {
        SceneParticipantUuid(value)
    }
}
