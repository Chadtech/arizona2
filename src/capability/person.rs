use crate::domain::{person_name::PersonName, person_uuid::PersonUuid};
use async_trait::async_trait;

pub struct NewPerson {
    pub person_uuid: PersonUuid,
    pub person_name: PersonName,
}

#[async_trait]
pub trait PersonCapability {
    async fn create_person(&self, new_person: NewPerson) -> Result<PersonUuid, String>;
}
