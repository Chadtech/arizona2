pub struct SceneUuid(uuid::Uuid);

impl SceneUuid {
    pub fn new() -> Self {
        SceneUuid(uuid::Uuid::now_v7())
    }
    pub fn to_uuid(&self) -> uuid::Uuid {
        self.0
    }
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        SceneUuid(uuid)
    }
}
