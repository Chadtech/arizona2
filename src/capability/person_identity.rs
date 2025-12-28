use async_trait::async_trait;

use crate::domain::person_identity_uuid::PersonIdentityUuid;
use crate::domain::person_uuid::PersonUuid;

pub struct NewPersonIdentity {
    pub person_identity_uuid: PersonIdentityUuid,
    pub person_name: String,
    pub identity: String,
}

#[async_trait]
pub trait PersonIdentityCapability {
    async fn create_person_identity(
        &self,
        new_person_identity: NewPersonIdentity,
    ) -> Result<PersonIdentityUuid, String>;
    async fn get_person_identity(&self, person_uuid: &PersonUuid)
        -> Result<Option<String>, String>;
}
