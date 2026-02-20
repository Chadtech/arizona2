use crate::capability::motivation::{MotivationCapability, NewMotivation};
use crate::domain::motivation::Motivation;
use crate::domain::motivation_uuid::MotivationUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::worker::Worker;
use chrono::{DateTime, Utc};
use sqlx::Row;

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
        person_uuid: PersonUuid,
    ) -> Result<Vec<Motivation>, String> {
        let rows = sqlx::query(
            r#"
                SELECT uuid, person_uuid, content, priority, created_at, ended_at, deleted_at
                FROM motivation
                WHERE person_uuid = $1::UUID
                  AND deleted_at IS NULL
                ORDER BY priority DESC, created_at DESC
            "#,
        )
        .bind(person_uuid.to_uuid())
        .fetch_all(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching motivations: {}", err))?;

        let mut motivations = Vec::with_capacity(rows.len());
        for row in rows {
            motivations.push(Motivation {
                uuid: MotivationUuid::from_uuid(
                    row.try_get::<uuid::Uuid, _>("uuid")
                        .map_err(|err| format!("Error reading motivation uuid: {}", err))?,
                ),
                person_uuid: PersonUuid::from_uuid(
                    row.try_get::<uuid::Uuid, _>("person_uuid")
                        .map_err(|err| format!("Error reading person uuid: {}", err))?,
                ),
                content: row
                    .try_get::<String, _>("content")
                    .map_err(|err| format!("Error reading motivation content: {}", err))?,
                priority: row
                    .try_get::<i32, _>("priority")
                    .map_err(|err| format!("Error reading motivation priority: {}", err))?,
                created_at: row
                    .try_get::<DateTime<Utc>, _>("created_at")
                    .map_err(|err| format!("Error reading motivation created_at: {}", err))?,
                ended_at: row
                    .try_get::<Option<DateTime<Utc>>, _>("ended_at")
                    .map_err(|err| format!("Error reading motivation ended_at: {}", err))?,
                deleted_at: row
                    .try_get::<Option<DateTime<Utc>>, _>("deleted_at")
                    .map_err(|err| format!("Error reading motivation deleted_at: {}", err))?,
            });
        }

        Ok(motivations)
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
