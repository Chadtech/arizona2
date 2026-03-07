use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PersonIdentityUuid(Uuid);

impl PersonIdentityUuid {
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
