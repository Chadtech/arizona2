use std::fmt::Display;

pub enum Model {
    Gpt4p1,
}

impl Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Model::Gpt4p1 => "gpt-4o-2024-08-06".to_string(),
            }
        )
    }
}
