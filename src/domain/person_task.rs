use crate::domain::person_task_uuid::PersonTaskUuid;
use crate::domain::person_uuid::PersonUuid;
use chrono::{DateTime, Utc};
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct PersonTask {
    pub uuid: PersonTaskUuid,
    pub person_uuid: PersonUuid,
    pub content: String,
    pub state: Option<String>,
    pub success_condition: Option<String>,
    pub abandon_condition: Option<String>,
    pub failure_condition: Option<String>,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub abandoned_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersonTaskTerminalOutcome {
    Completed,
    Failed,
    Abandoned,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersonTaskOutcomeCheck {
    StillActive,
    Terminal(PersonTaskTerminalOutcome),
}

impl PersonTaskTerminalOutcome {
    pub fn to_name(&self) -> String {
        match self {
            PersonTaskTerminalOutcome::Completed => "completed".to_string(),
            PersonTaskTerminalOutcome::Failed => "failed".to_string(),
            PersonTaskTerminalOutcome::Abandoned => "abandoned".to_string(),
        }
    }

    pub fn all() -> Vec<PersonTaskTerminalOutcome> {
        vec![
            PersonTaskTerminalOutcome::Completed,
            PersonTaskTerminalOutcome::Failed,
            PersonTaskTerminalOutcome::Abandoned,
        ]
    }

    pub fn all_names() -> Vec<String> {
        Self::all()
            .iter()
            .map(|outcome| outcome.to_name())
            .collect()
    }

    pub fn from_tool_value(value: &str) -> Result<Self, String> {
        match PersonTaskTerminalOutcome::all()
            .iter()
            .find(|terminal_outcome| terminal_outcome.to_name() == value)
        {
            Some(terminal_outcome) => Ok(terminal_outcome.clone()),
            None => Err(format!(
                "Unrecognized person task terminal outcome: {}. Must be one of: {}",
                value,
                PersonTaskTerminalOutcome::all_names().join(", ")
            )),
        }
    }
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
