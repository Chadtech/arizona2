use crate::domain::{person_name::PersonName, person_uuid::PersonUuid};

pub struct NewPerson {
    pub person_uuid: PersonUuid,
    pub person_name: PersonName,
}

pub trait PersonCapability {
    async fn create_person(&self, new_person: NewPerson) -> Result<PersonUuid, String>;
    async fn get_persons_name(&self, person_uuid: PersonUuid) -> Result<PersonName, String>;
    async fn get_person_uuid_by_name(&self, person_name: PersonName) -> Result<PersonUuid, String>;
    async fn set_person_hibernating(
        &self,
        person_uuid: &PersonUuid,
        is_hibernating: bool,
    ) -> Result<(), String>;
    async fn is_person_hibernating(&self, person_uuid: &PersonUuid) -> Result<bool, String>;
    async fn set_person_enabled(
        &self,
        person_uuid: &PersonUuid,
        is_enabled: bool,
    ) -> Result<(), String>;
    async fn is_person_enabled(&self, person_uuid: &PersonUuid) -> Result<bool, String>;
}
