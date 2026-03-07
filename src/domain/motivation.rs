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
    pub fn many_to_list_text(motivations: &[Motivation]) -> String {
        if motivations.is_empty() {
            "None.".to_string()
        } else {
            motivations
                .iter()
                .map(|motivation| motivation.to_list_text())
                .collect::<Vec<String>>()
                .join("\n")
        }
    }

    fn to_list_text(&self) -> String {
        format!("- (priority {}) {}", self.priority, self.content)
    }
}
