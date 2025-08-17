use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PersonIdentityUuid(Uuid);

impl PersonIdentityUuid {
    pub fn from_str(s: &str) -> Result<Self, uuid::Error> {
        Uuid::parse_str(s).map(Self)
    }

    pub fn from_uuid(u: Uuid) -> Self {
        Self(u)
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}
