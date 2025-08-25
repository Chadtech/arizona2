use async_trait::async_trait;

use crate::capability::person_identity::{NewPersonIdentity, PersonIdentityCapability};
use crate::domain::person_identity_uuid::PersonIdentityUuid;
use crate::worker::Worker;

#[async_trait]
impl PersonIdentityCapability for Worker {
    async fn create_person_identity(
        &self,
        new_person_identity: NewPersonIdentity,
    ) -> Result<PersonIdentityUuid, String> {
        let ret = sqlx::query!(
            r#"
                INSERT INTO person_identity (uuid, person_uuid, identity)
                SELECT $1::UUID, person.uuid, $2::TEXT
                FROM person
                WHERE name = $3::TEXT
                RETURNING uuid;
            "#,
            new_person_identity.person_identity_uuid.to_uuid(),
            new_person_identity.identity,
            new_person_identity.person_name
        )
        .fetch_one(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting new person identity: {}", err))?;

        Ok(PersonIdentityUuid::from_uuid(ret.uuid))
    }
}
