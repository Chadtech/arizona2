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
    ) -> Result<(), String> {
        let _ = person_uuid;
        let _ = is_hibernating;
        Err("set_person_hibernating is not implemented".to_string())
    }
    async fn is_person_hibernating(&self, person_uuid: &PersonUuid) -> Result<bool, String> {
        let _ = person_uuid;
        Ok(false)
    }
}
