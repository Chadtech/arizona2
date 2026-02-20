use crate::capability::job_runner_settings::JobRunnerSettingsCapability;
use crate::worker::Worker;
use async_trait::async_trait;
use sqlx::Row;

#[async_trait]
impl JobRunnerSettingsCapability for Worker {
    async fn get_job_runner_poll_interval_secs(&self) -> Result<u64, String> {
        let row = sqlx::query(
            r#"
                SELECT poll_interval_secs
                FROM job_runner_setting
                WHERE id = TRUE;
            "#,
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching job runner poll interval: {}", err))?;

        let row = match row {
            Some(row) => row,
            None => {
                return Err("Job runner poll interval is missing from job_runner_setting".to_string());
            }
        };

        let secs: i64 = row
            .try_get::<i64, _>("poll_interval_secs")
            .map_err(|err| format!("Error reading job runner poll interval: {}", err))?;

        if secs <= 0 {
            return Err(format!(
                "Job runner poll interval must be positive, got {}",
                secs
            ));
        }

        Ok(secs as u64)
    }

    async fn set_job_runner_poll_interval_secs(&self, secs: u64) -> Result<(), String> {
        if secs == 0 {
            return Err("Job runner poll interval must be positive".to_string());
        }

        sqlx::query(
            r#"
                UPDATE job_runner_setting
                SET poll_interval_secs = $1
                WHERE id = TRUE;
            "#,
        )
        .bind(secs as i64)
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error updating job runner poll interval: {}", err))?;

        Ok(())
    }
}
