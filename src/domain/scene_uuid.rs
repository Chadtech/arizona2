use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneUuid(uuid::Uuid);

impl Display for SceneUuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

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

impl From<uuid::Uuid> for SceneUuid {
    fn from(value: uuid::Uuid) -> Self {
        SceneUuid(value)
    }
}
