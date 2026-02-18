use crate::domain::goal_uuid::GoalUuid;
use crate::domain::person_uuid::PersonUuid;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct Goal {
    pub uuid: GoalUuid,
    pub person_uuid: PersonUuid,
    pub content: String,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}
