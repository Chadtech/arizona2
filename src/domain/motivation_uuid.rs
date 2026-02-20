use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct MotivationUuid(Uuid);

impl MotivationUuid {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn to_uuid(&self) -> Uuid {
        self.0
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}
