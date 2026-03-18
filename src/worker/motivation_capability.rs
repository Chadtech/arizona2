use crate::capability::motivation::{MotivationCapability, NewMotivation};
use crate::domain::motivation::Motivation;
use crate::domain::motivation_uuid::MotivationUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::worker::Worker;

impl MotivationCapability for Worker {
    async fn create_motivation(
        &self,
        new_motivation: NewMotivation,
    ) -> Result<MotivationUuid, String> {
        let motivation_uuid = MotivationUuid::new();
        sqlx::query(
            r#"
                INSERT INTO motivation (uuid, person_uuid, content, priority)
                VALUES ($1::UUID, $2::UUID, $3::TEXT, $4::INTEGER)
            "#,
        )
        .bind(motivation_uuid.to_uuid())
        .bind(new_motivation.person_uuid.to_uuid())
        .bind(new_motivation.content)
        .bind(new_motivation.priority)
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting motivation: {}", err))?;

        Ok(motivation_uuid)
    }

    async fn get_motivations_for_person(
        &self,
        person_uuid: &PersonUuid,
    ) -> Result<Vec<Motivation>, String> {
        let rows = sqlx::query!(
            r#"
                SELECT uuid, content, priority, created_at, ended_at
                FROM motivation
                WHERE person_uuid = $1::UUID
                  AND deleted_at IS NULL
                ORDER BY priority DESC, created_at DESC
            "#,
            person_uuid.to_uuid()
        )
        .fetch_all(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching motivations: {}", err))?;

        Ok(rows
            .into_iter()
            .map(|row| Motivation {
                uuid: MotivationUuid::from_uuid(row.uuid),
                content: row.content,
                priority: row.priority,
                created_at: row.created_at,
                ended_at: row.ended_at,
            })
            .collect())
    }

    async fn delete_motivation(&self, motivation_uuid: MotivationUuid) -> Result<(), String> {
        sqlx::query(
            r#"
                UPDATE motivation
                SET deleted_at = NOW()
                WHERE uuid = $1::UUID
            "#,
        )
        .bind(motivation_uuid.to_uuid())
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error deleting motivation: {}", err))?;

        Ok(())
    }
}
