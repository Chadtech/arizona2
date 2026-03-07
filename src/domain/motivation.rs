use crate::domain::motivation_uuid::MotivationUuid;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct Motivation {
    pub uuid: MotivationUuid,
    pub content: String,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

impl Motivation {
    pub fn to_list_text(motivations: &Vec<Motivation>) -> String {
        if motivations.is_empty() {
            "None.".to_string()
        } else {
            motivations
                .iter()
                .map(|motivation| {
                    format!(
                        "- (priority {}) {}",
                        motivation.priority, motivation.content
                    )
                })
                .collect::<Vec<String>>()
                .join("\n")
        }
    }
}
