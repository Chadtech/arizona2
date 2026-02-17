use serde_json::Value;

pub trait LogEventCapability {
    async fn log_event(
        &self,
        event_name: String,
        data: Option<Value>,
    ) -> Result<(), String>;
}
