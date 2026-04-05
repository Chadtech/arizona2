use crate::domain::person_task_uuid::PersonTaskUuid;
use crate::domain::person_uuid::PersonUuid;
use chrono::{DateTime, Utc};
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct PersonTask {
    pub uuid: PersonTaskUuid,
    pub person_uuid: PersonUuid,
    pub content: String,
    pub success_condition: Option<String>,
    pub abandon_condition: Option<String>,
    pub failure_condition: Option<String>,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub abandoned_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
}

impl Display for PersonTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Task (priority {} / 100):\n{}",
            self.priority, self.content,
        )
    }
}
