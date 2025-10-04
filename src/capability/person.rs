use crate::domain::{person_name::PersonName, person_uuid::PersonUuid};

pub struct NewPerson {
    pub person_uuid: PersonUuid,
    pub person_name: PersonName,
}

pub trait PersonCapability {
    async fn create_person(&self, new_person: NewPerson) -> Result<PersonUuid, String>;
}
