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
                INSERT INTO person_identity (person_uuid, identity)
                SELECT person.uuid, $1::TEXT
                FROM person
                WHERE name = $2::TEXT
                RETURNING uuid;
            "#,
            new_person_identity.identity,
            new_person_identity.person_name
        )
        .fetch_one(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting new person identity: {}", err))?;

        Ok(PersonIdentityUuid::from_uuid(ret.uuid))
    }
}
