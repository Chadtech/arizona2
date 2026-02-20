use crate::capability::reaction_history::ReactionHistoryCapability;
use crate::domain::person_uuid::PersonUuid;
use crate::worker::Worker;
use chrono::{DateTime, Utc};
use uuid::Uuid;

impl ReactionHistoryCapability for Worker {
    async fn record_reaction(
        &self,
        person_uuid: &PersonUuid,
        action_kind: &str,
    ) -> Result<(), String> {
        let reaction_uuid = Uuid::now_v7();

        sqlx::query(
            r#"
                INSERT INTO reaction_history (uuid, person_uuid, action_kind)
                VALUES ($1::UUID, $2::UUID, $3::TEXT);
            "#,
        )
        .bind(reaction_uuid)
        .bind(person_uuid.to_uuid())
        .bind(action_kind)
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting reaction history: {}", err))?;

        Ok(())
    }

    async fn has_reacted_since(
        &self,
        person_uuid: &PersonUuid,
        since: DateTime<Utc>,
    ) -> Result<bool, String> {
        let row = sqlx::query(
            r#"
                SELECT 1
                FROM reaction_history
                WHERE person_uuid = $1::UUID
                  AND created_at >= $2
                LIMIT 1;
            "#,
        )
        .bind(person_uuid.to_uuid())
        .bind(since)
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error querying reaction history: {}", err))?;

        Ok(row.is_some())
    }
}
