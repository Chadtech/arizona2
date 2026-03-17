use crate::capability::person::{NewPerson, PersonCapability};
use crate::domain::person_name::PersonName;
use crate::domain::person_uuid::PersonUuid;
use crate::worker::Worker;
use sqlx::Row;

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

    async fn get_all_person_uuids(&self) -> Result<Vec<PersonUuid>, String> {
        let rows = sqlx::query!(
            r#"
                SELECT uuid
                FROM person
                ORDER BY name ASC;
            "#
        )
        .fetch_all(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching all person UUIDs: {}", err))?;

        Ok(rows
            .into_iter()
            .map(|row| PersonUuid::from_uuid(row.uuid))
            .collect())
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

    async fn set_person_hibernating(
        &self,
        person_uuid: &PersonUuid,
        is_hibernating: bool,
    ) -> Result<(), String> {
        sqlx::query(
            r#"
                UPDATE person
                SET is_hibernating = $2::BOOLEAN,
                    updated_at = NOW()
                WHERE uuid = $1::UUID;
            "#,
        )
        .bind(person_uuid.to_uuid())
        .bind(is_hibernating)
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error updating hibernation state: {}", err))?;

        Ok(())
    }

    async fn is_person_hibernating(&self, person_uuid: &PersonUuid) -> Result<bool, String> {
        let rec = sqlx::query(
            r#"
                SELECT is_hibernating
                FROM person
                WHERE uuid = $1::UUID;
            "#,
        )
        .bind(person_uuid.to_uuid())
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching hibernation state: {}", err))?;

        match rec {
            Some(row) => row
                .try_get::<bool, _>("is_hibernating")
                .map_err(|err| format!("Error reading is_hibernating: {}", err)),
            None => Err(format!("Person {} not found", person_uuid.to_uuid())),
        }
    }

    async fn set_person_enabled(
        &self,
        person_uuid: &PersonUuid,
        is_enabled: bool,
    ) -> Result<(), String> {
        sqlx::query!(
            r#"
                UPDATE person
                SET is_enabled = $2::BOOLEAN,
                    updated_at = NOW()
                WHERE uuid = $1::UUID;
            "#,
            person_uuid.to_uuid(),
            is_enabled
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error updating enabled state: {}", err))?;

        Ok(())
    }

    async fn is_person_enabled(&self, person_uuid: &PersonUuid) -> Result<bool, String> {
        let rec = sqlx::query!(
            r#"
                SELECT is_enabled
                FROM person
                WHERE uuid = $1::UUID;
            "#,
            person_uuid.to_uuid()
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching enabled state: {}", err))?;

        match rec {
            Some(row) => Ok(row.is_enabled),
            None => Err(format!("Person {} not found", person_uuid.to_uuid())),
        }
    }
}
