use crate::capability::log_event::LogEventCapability;
use crate::worker::Worker;
use uuid::Uuid;

impl LogEventCapability for Worker {
    async fn log_event(
        &self,
        event_name: String,
        data: Option<serde_json::Value>,
    ) -> Result<(), String> {
        let event_uuid = Uuid::now_v7();

        sqlx::query(
            r#"
                INSERT INTO log_event (uuid, event_name, data)
                VALUES ($1::UUID, $2::TEXT, $3::JSONB)
            "#,
        )
        .bind(event_uuid)
        .bind(event_name)
        .bind(data)
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error inserting log event: {}", err))?;

        Ok(())
    }
}
