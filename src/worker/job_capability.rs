use crate::capability::job::JobCapability;
use crate::domain::job::{Job, JobKind};
use crate::domain::job_uuid::JobUuid;
use crate::nice_display::NiceDisplay;
use crate::worker::Worker;

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

    async fn pop_job(&self) -> Result<Option<Job>, String> {
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
                let job =
                    Job::parse(JobUuid::from_uuid(job_uuid), ret_rec.name).map_err(|err| {
                        format!("Error parsing job\n{}", err.to_nice_error().to_string())
                    })?;

                Ok(Some(job))
            }
        }
    }
}
