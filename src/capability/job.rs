use crate::domain::job::{Job, JobKind};

pub trait JobCapability {
    async fn unshift_job(&self, job: JobKind) -> Result<(), String>;
    async fn pop_job(&self) -> Result<Option<Job>, String>;
}
