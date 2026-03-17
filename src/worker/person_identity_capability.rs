use async_trait::async_trait;

use crate::capability::person_identity::{NewPersonIdentity, PersonIdentityCapability};
use crate::domain::person_identity_uuid::PersonIdentityUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::nice_display::NiceDisplay;
use crate::open_ai::completion::Completion;
use crate::open_ai::role::Role;
use crate::worker::Worker;

#[async_trait]
impl PersonIdentityCapability for Worker {
    async fn summarize_person_identity(
        &self,
        person_name: &str,
        identity: &str,
    ) -> Result<String, String> {
        let mut completion = Completion::new();
        completion.add_message(
            Role::System,
            "Summarize this description of a person's identity in no more than three sentences.",
        );
        completion.add_message(
            Role::User,
            format!(
                "Person name: {}\n\nIdentity text:\n{}",
                person_name, identity
            )
            .as_str(),
        );

        let response = completion
            .send_request(&self.open_ai_key, self.reqwest_client.clone())
            .await
            .map_err(|err| err.message())?;

        response.as_message().map_err(|err| err.message())
    }

    async fn create_person_identity(
        &self,
        new_person_identity: NewPersonIdentity,
    ) -> Result<PersonIdentityUuid, String> {
        let summary = self
            .summarize_person_identity(
                new_person_identity.person_name.as_str(),
                new_person_identity.identity.as_str(),
            )
            .await
            .map_err(|err| format!("Error summarizing person identity: {}", err))?;

        let ret = sqlx::query!(
            r#"
                INSERT INTO person_identity (uuid, person_uuid, identity, summary)
                SELECT $1::UUID, person.uuid, $2::TEXT, $3::TEXT
                FROM person
                WHERE name = $4::TEXT
                RETURNING uuid;
            "#,
            new_person_identity.person_identity_uuid.to_uuid(),
            new_person_identity.identity,
            summary,
            new_person_identity.person_name
        )
        .fetch_one(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting new person identity: {}", err))?;

        Ok(PersonIdentityUuid::from_uuid(ret.uuid))
    }

    async fn get_person_identity(
        &self,
        person_uuid: &PersonUuid,
    ) -> Result<Option<String>, String> {
        let rec = sqlx::query!(
            r#"
                SELECT identity
                FROM person_identity
                WHERE person_uuid = $1::UUID
                ORDER BY created_at DESC
                LIMIT 1;
            "#,
            person_uuid.to_uuid()
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching person identity: {}", err))?;

        Ok(rec.map(|r| r.identity))
    }

    async fn get_person_identity_summary(
        &self,
        person_uuid: &PersonUuid,
    ) -> Result<Option<String>, String> {
        let rec = sqlx::query!(
            r#"
                SELECT summary
                FROM person_identity
                WHERE person_uuid = $1::UUID
                ORDER BY created_at DESC
                LIMIT 1;
            "#,
            person_uuid.to_uuid()
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching person identity summary: {}", err))?;

        Ok(rec.and_then(|r| r.summary))
    }
}
