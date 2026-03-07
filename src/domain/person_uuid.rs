use serde::{Deserialize, Serialize};
use std::fmt::Display;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonUuid(Uuid);

impl PersonUuid {
    pub fn from_uuid(u: Uuid) -> Self {
        Self(u)
    }

    pub fn to_uuid(&self) -> Uuid {
        self.0
    }

    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Display for PersonUuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = format!("Person/{}", self.0);
        write!(f, "{}", s)
    }
}
