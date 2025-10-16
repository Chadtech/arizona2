use crate::capability::job::JobCapability;
use crate::domain::job::{Job, JobKind, PoppedJob};
use crate::domain::job_uuid::JobUuid;
use crate::nice_display::NiceDisplay;
use crate::worker::Worker;
use chrono::Utc;
use sqlx::Row;

impl JobCapability for Worker {
    async fn unshift_job(&self, job: JobKind) -> Result<(), String> {
        let job_uuid = JobUuid::new();
        sqlx::query!(
            r#"
				INSERT INTO job (uuid, name)
				VALUES ($1::UUID, $2::TEXT);
			"#,
            job_uuid.to_uuid(),
            job.to_name()
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error unshifting new job: {}", err))?;

        Ok(())
    }

    async fn pop_next_job(&self) -> Result<Option<PoppedJob>, String> {
        let rec = sqlx::query!(
            r#"
                UPDATE job
                SET started_at = NOW()
                WHERE uuid = (
                    SELECT uuid
                    FROM job
                    WHERE started_at IS NULL
                      AND finished_at IS NULL
                    ORDER BY created_at DESC
                    LIMIT 1
                )
                RETURNING uuid;
            "#
        )
        .fetch_one(&self.sqlx)
        .await
        .map_err(|err| format!("Error setting started_at on job: {}", err))?;

        dbg!(&rec);

        if rec.uuid.is_nil() {
            return Ok(None);
        }

        let job_uuid = rec.uuid;

        let maybe_job_ret = sqlx::query!(
            r#"
                SELECT name
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
                let job = PoppedJob::parse(JobUuid::from_uuid(job_uuid), ret_rec.name).map_err(
                    |err| format!("Error parsing job\n{}", err.to_nice_error().to_string()),
                )?;

                Ok(Some(job))
            }
        }
    }

    async fn recent_jobs(&self, limit: i64) -> Result<Vec<Job>, String> {
        let rows = sqlx::query(
            r#"
                SELECT uuid, name, finished_at
                FROM job
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

            let finished_at = row
                .try_get::<Option<chrono::DateTime<Utc>>, _>("finished_at")
                .map_err(|err| format!("Error reading finished_at from row: {}", err))?
                .map(|dt| dt.timestamp());

            let job = Job::parse(JobUuid::from_uuid(uuid), finished_at, name)
                .map_err(|err| format!("Error parsing job\n{}", err.to_nice_error().to_string()))?;
            jobs.push(job);
        }

        Ok(jobs)
    }

    async fn mark_job_started(&self, job_uuid: &JobUuid) -> Result<(), String> {
        sqlx::query!(
            r#"
                UPDATE job
                SET started_at = NOW()
                WHERE uuid = $1::UUID;
            "#,
            job_uuid.to_uuid()
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error marking job as started: {}", err))?;

        Ok(())
    }

    async fn mark_job_finished(&self, job_uuid: &JobUuid) -> Result<(), String> {
        sqlx::query!(
            r#"
                UPDATE job
                SET finished_at = NOW()
                WHERE uuid = $1::UUID;
            "#,
            job_uuid.to_uuid()
        )
        .execute(&self.sqlx)
        .await
        .map_err(|err| format!("Error marking job as finished: {}", err))?;

        Ok(())
    }
}
