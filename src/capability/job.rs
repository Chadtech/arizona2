use crate::domain::job::{Job, JobKind, PoppedJob};
use crate::domain::job_uuid::JobUuid;

pub trait JobCapability {
    async fn unshift_job(&self, job: JobKind) -> Result<(), String>;
    async fn pop_next_job(&self) -> Result<Option<PoppedJob>, String>;
    async fn recent_jobs(&self, limit: i64) -> Result<Vec<Job>, String>;
    async fn mark_job_finished(&self, job_uuid: &JobUuid) -> Result<(), String>;
    async fn mark_job_failed(&self, job_uuid: &JobUuid, details: &str) -> Result<(), String>;
}
