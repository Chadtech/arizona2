use async_trait::async_trait;

#[async_trait]
pub trait JobRunnerSettingsCapability {
    async fn get_job_runner_poll_interval_secs(&self) -> Result<u64, String>;
    async fn set_job_runner_poll_interval_secs(&self, secs: u64) -> Result<(), String>;
}
