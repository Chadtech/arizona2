use crate::capability::goal::{GoalCapability, NewGoal};
use crate::domain::goal::Goal;
use crate::domain::goal_uuid::GoalUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::worker::Worker;
use chrono::{DateTime, Utc};
use sqlx::Row;

impl GoalCapability for Worker {
    async fn create_goal(&self, new_goal: NewGoal) -> Result<GoalUuid, String> {
        let goal_uuid = GoalUuid::new();
        sqlx::query(
            r#"
                INSERT INTO goal (uuid, person_uuid, content, priority)
                VALUES ($1::UUID, $2::UUID, $3::TEXT, $4::INTEGER)
            "#,
        )
        .bind(goal_uuid.to_uuid())
        .bind(new_goal.person_uuid.to_uuid())
        .bind(new_goal.content)
        .bind(new_goal.priority)
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting goal: {}", err))?;

        Ok(goal_uuid)
    }

    async fn get_goals_for_person(
        &self,
        person_uuid: PersonUuid,
    ) -> Result<Vec<Goal>, String> {
        let rows = sqlx::query(
            r#"
                SELECT uuid, person_uuid, content, priority, created_at, ended_at, deleted_at
                FROM goal
                WHERE person_uuid = $1::UUID
                  AND deleted_at IS NULL
                ORDER BY priority DESC, created_at DESC
            "#,
        )
        .bind(person_uuid.to_uuid())
        .fetch_all(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching goals: {}", err))?;

        let mut goals = Vec::with_capacity(rows.len());
        for row in rows {
            goals.push(Goal {
                uuid: GoalUuid::from_uuid(
                    row.try_get::<uuid::Uuid, _>("uuid")
                        .map_err(|err| format!("Error reading goal uuid: {}", err))?,
                ),
                person_uuid: PersonUuid::from_uuid(
                    row.try_get::<uuid::Uuid, _>("person_uuid")
                        .map_err(|err| format!("Error reading person uuid: {}", err))?,
                ),
                content: row
                    .try_get::<String, _>("content")
                    .map_err(|err| format!("Error reading goal content: {}", err))?,
                priority: row
                    .try_get::<i32, _>("priority")
                    .map_err(|err| format!("Error reading goal priority: {}", err))?,
                created_at: row
                    .try_get::<DateTime<Utc>, _>("created_at")
                    .map_err(|err| format!("Error reading goal created_at: {}", err))?,
                ended_at: row
                    .try_get::<Option<DateTime<Utc>>, _>("ended_at")
                    .map_err(|err| format!("Error reading goal ended_at: {}", err))?,
                deleted_at: row
                    .try_get::<Option<DateTime<Utc>>, _>("deleted_at")
                    .map_err(|err| format!("Error reading goal deleted_at: {}", err))?,
            });
        }

        Ok(goals)
    }

    async fn delete_goal(&self, goal_uuid: GoalUuid) -> Result<(), String> {
        sqlx::query(
            r#"
                UPDATE goal
                SET deleted_at = NOW()
                WHERE uuid = $1::UUID
            "#,
        )
        .bind(goal_uuid.to_uuid())
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error deleting goal: {}", err))?;

        Ok(())
    }
}
