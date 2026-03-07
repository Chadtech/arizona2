use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonUuid(Uuid);

impl PersonUuid {
    pub fn from_uuid(u: Uuid) -> Self {
        Self(u)
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }

    pub fn to_uuid(&self) -> Uuid {
        self.0
    }

    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}
