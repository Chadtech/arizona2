use crate::domain::motivation_uuid::MotivationUuid;
use crate::domain::person_uuid::PersonUuid;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct Motivation {
    pub uuid: MotivationUuid,
    pub person_uuid: PersonUuid,
    pub content: String,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}
