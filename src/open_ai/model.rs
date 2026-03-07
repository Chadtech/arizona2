use std::fmt::Display;

#[allow(dead_code)]
pub enum Model {
    Gpt4o,
    Gpt5Mini,
}

impl Model {
    pub const DEFAULT: Model = Model::Gpt5Mini;
}

impl Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Model::Gpt4o => "gpt-4o-2024-08-06".to_string(),
                Model::Gpt5Mini => "gpt-5-mini".to_string(),
            }
        )
    }
}
