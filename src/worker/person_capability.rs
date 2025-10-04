use crate::capability::person::{NewPerson, PersonCapability};
use crate::domain::person_uuid::PersonUuid;
use crate::worker::Worker;

impl PersonCapability for Worker {
    async fn create_person(&self, new_person: NewPerson) -> Result<PersonUuid, String> {
        let ret = sqlx::query!(
            r#"
                INSERT INTO person (uuid, name)
                VALUES ($1::UUID, $2::TEXT)
                RETURNING uuid;
            "#,
            new_person.person_uuid.to_uuid(),
            new_person.person_name.to_string()
        )
        .fetch_one(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting new person identity: {}", err))?;

        Ok(PersonUuid::from_uuid(ret.uuid))
    }
}
