use crate::capability::person::{NewPerson, PersonCapability};
use crate::domain::person_name::PersonName;
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
            new_person.person_name.as_str()
        )
        .fetch_one(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting new person identity: {}", err))?;

        Ok(PersonUuid::from_uuid(ret.uuid))
    }

    async fn get_persons_name(&self, person_uuid: PersonUuid) -> Result<PersonName, String> {
        let rec = sqlx::query!(
            r#"
                SELECT name
                FROM person
                WHERE uuid = $1::UUID;
            "#,
            person_uuid.to_uuid()
        )
        .fetch_one(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching person's name: {}", err))?;

        Ok(PersonName::from_string(rec.name))
    }

    async fn get_person_uuid_by_name(&self, person_name: PersonName) -> Result<PersonUuid, String> {
        let rec = sqlx::query!(
            r#"
                SELECT uuid
                FROM person
                WHERE name = $1::TEXT;
            "#,
            person_name.as_str()
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching person UUID: {}", err))?;

        let rec = rec.ok_or_else(|| format!("Person '{}' not found", person_name.as_str()))?;

        Ok(PersonUuid::from_uuid(rec.uuid))
    }
}
