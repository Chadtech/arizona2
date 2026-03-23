use std::fmt::Display;

pub struct PersonTask {
    pub content: String,
    pub success_condition: Option<String>,
    pub abandon_condition: Option<String>,
    pub failure_condition: Option<String>,
    pub priority: i32,
}

impl PersonTask {
    pub fn dev() -> PersonTask {
        PersonTask {
            content: "Befriend Chadtech".to_string(),
            success_condition: Some("Chadtech is your friend".to_string()),
            abandon_condition: None,
            failure_condition: None,
            priority: 100,
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
