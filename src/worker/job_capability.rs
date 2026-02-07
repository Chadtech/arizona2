use crate::capability::job::JobCapability;
use crate::domain::job::{Job, JobKind, PoppedJob};
use crate::domain::job_uuid::JobUuid;
use crate::nice_display::NiceDisplay;
use crate::worker::Worker;
use chrono::{DateTime, Utc};
use sqlx::Row;

impl JobCapability for Worker {
    async fn unshift_job(&self, job: JobKind) -> Result<(), String> {
        let job_uuid = JobUuid::new();
        let job_name = job.to_name();
        let job_data = job.to_data()?;
        let run_at_active_ms = match &job {
            JobKind::PersonWaiting(wait_job) => Some(wait_job.run_at_active_ms()),
            _ => None,
        };

        sqlx::query(
            r#"
				INSERT INTO job (uuid, name, data, run_at_active_ms)
				VALUES ($1::UUID, $2::TEXT, $3::JSONB, $4::BIGINT);
			"#,
        )
        .bind(job_uuid.to_uuid()?)
        .bind(job_name)
        .bind(job_data)
        .bind(run_at_active_ms)
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error unshifting new job: {}", err))?;

        Ok(())
    }

    async fn pop_next_job(&self, current_active_ms: i64) -> Result<Option<PoppedJob>, String> {
        let maybe_rec = sqlx::query(
            r#"
                UPDATE job
                SET started_at = NOW()
                WHERE uuid = (
                    SELECT uuid
                    FROM job
                WHERE started_at IS NULL
                  AND finished_at IS NULL
                  AND deleted_at IS NULL
                  AND (run_at_active_ms IS NULL OR run_at_active_ms <= $1)
                ORDER BY created_at ASC
                LIMIT 1
            )
                RETURNING uuid;
            "#,
        )
        .bind(current_active_ms)
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error setting started_at on job: {}", err))?;

        let rec = match maybe_rec {
            None => return Ok(None),
            Some(r) => r,
        };

        let job_uuid: uuid::Uuid = rec
            .try_get::<uuid::Uuid, _>("uuid")
            .map_err(|err| format!("Error reading uuid from row: {}", err))?;

        let maybe_job_ret = sqlx::query!(
            r#"
                SELECT name, data
                FROM job
                WHERE uuid = $1::UUID;
            "#,
            job_uuid
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching popped job details: {}", err))?;

        match maybe_job_ret {
            None => Ok(None),
            Some(ret_rec) => {
                let job =
                    PoppedJob::parse(JobUuid::from_uuid(job_uuid), ret_rec.name, ret_rec.data)
                        .map_err(|err| {
                            format!("Error parsing job\n{}", err.to_nice_error().to_string())
                        })?;

                Ok(Some(job))
            }
        }
    }

    async fn recent_jobs(&self, limit: i64) -> Result<Vec<Job>, String> {
        let rows = sqlx::query(
            r#"
                SELECT uuid, name, started_at, finished_at, error, deleted_at, data
                FROM job
                WHERE deleted_at IS NULL
                ORDER BY created_at DESC
                LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching recent jobs: {}", err))?;

        let mut jobs = Vec::with_capacity(rows.len());
        for row in rows {
            // Use dynamic row getters to avoid requiring `sqlx::query!` offline preparation
            let uuid: uuid::Uuid = row
                .try_get::<uuid::Uuid, _>("uuid")
                .map_err(|err| format!("Error reading uuid from row: {}", err))?;

            let name: String = row
                .try_get::<String, _>("name")
                .map_err(|err| format!("Error reading name from row: {}", err))?;

            let started_at = row
                .try_get::<Option<DateTime<Utc>>, _>("started_at")
                .map_err(|err| format!("Error reading started_at from row: {}", err))?;

            let finished_at = row
                .try_get::<Option<DateTime<Utc>>, _>("finished_at")
                .map_err(|err| format!("Error reading finished_at from row: {}", err))?;

            let error = row
                .try_get::<Option<String>, _>("error")
                .map_err(|err| format!("Error reading error from row: {}", err))?;

            let deleted_at = row
                .try_get::<Option<DateTime<Utc>>, _>("deleted_at")
                .map_err(|err| format!("Error reading deleted_at from row: {}", err))?;

            let job_data: Option<serde_json::Value> = row
                .try_get::<Option<serde_json::Value>, _>("data")
                .map_err(|err| format!("Error reading job data from row: {}", err))?;

            let job = Job::parse(
                JobUuid::from_uuid(uuid),
                started_at,
                finished_at,
                error,
                deleted_at,
                name,
                job_data,
            )
            .map_err(|err| format!("Error parsing job\n{}", err.to_nice_error().to_string()))?;
            jobs.push(job);
        }

        Ok(jobs)
    }

    async fn get_job_by_uuid(&self, job_uuid: &JobUuid) -> Result<Option<Job>, String> {
        let row = sqlx::query!(
            r#"
                SELECT uuid, name, started_at, finished_at, error, deleted_at, data
                FROM job
                WHERE uuid = $1::UUID
                  AND deleted_at IS NULL
            "#,
            job_uuid.to_uuid()?
        )
        .fetch_optional(&self.sqlx)
        .await
        .map_err(|err| format!("Error fetching job by uuid: {}", err))?;

        let row = match row {
            Some(row) => row,
            None => return Ok(None),
        };

        let job = Job::parse(
            JobUuid::from_uuid(row.uuid),
            row.started_at,
            row.finished_at,
            row.error,
            row.deleted_at,
            row.name,
            row.data,
        )
        .map_err(|err| format!("Error parsing job\n{}", err.to_nice_error().to_string()))?;

        Ok(Some(job))
    }

    async fn mark_job_finished(&self, job_uuid: &JobUuid) -> Result<(), String> {
        sqlx::query!(
            r#"
                UPDATE job
                SET finished_at = NOW()
                WHERE uuid = $1::UUID;
            "#,
            job_uuid.to_uuid()?
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error marking job as finished: {}", err))?;

        Ok(())
    }

    async fn mark_job_failed(&self, job_uuid: &JobUuid, details: &str) -> Result<(), String> {
        sqlx::query!(
            r#"
                UPDATE job
                SET error = $2::TEXT
                WHERE uuid = $1::UUID;
            "#,
            job_uuid.to_uuid()?,
            details
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error marking job as failed: {}", err))?;

        Ok(())
    }

    async fn reset_job(&self, job_uuid: &JobUuid) -> Result<(), String> {
        sqlx::query(
            r#"
                UPDATE job
                SET started_at = NULL,
                    finished_at = NULL,
                    error = NULL
                WHERE uuid = $1::UUID;
            "#,
        )
        .bind(job_uuid.to_uuid()?)
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error resetting job: {}", err))?;

        Ok(())
    }

    async fn delete_job(&self, job_uuid: &JobUuid) -> Result<(), String> {
        sqlx::query!(
            r#"
                UPDATE job
                SET deleted_at = NOW()
                WHERE uuid = $1::UUID
            "#,
            job_uuid.to_uuid()?
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error deleting job: {}", err))?;

        Ok(())
    }
}
