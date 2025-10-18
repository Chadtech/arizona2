use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JobUuid {
    Real(Uuid),
    Test(u64),
}

impl JobUuid {
    pub fn new() -> Self {
        JobUuid::Real(Uuid::now_v7())
    }
    pub fn to_uuid(&self) -> Result<Uuid, String> {
        match self {
            JobUuid::Real(uuid) => Ok(*uuid),
            JobUuid::Test(test_id) => Err(format!("Cannot turn test id {} into a uuid", test_id)),
        }
    }
    pub fn from_uuid(uuid: Uuid) -> Self {
        JobUuid::Real(uuid)
    }

    pub fn test_id(id: u64) -> Self {
        JobUuid::Test(id)
    }
}
