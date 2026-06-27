use std::fmt::Display;

#[allow(dead_code)]
pub enum Model {
    Gpt4o,
    Gpt5Mini,
    Gpt5p5,
}

impl Model {
    pub const DEFAULT: Model = Model::Gpt5p5;
}

impl Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Model::Gpt4o => "gpt-4o-2024-08-06".to_string(),
                Model::Gpt5Mini => "gpt-5-mini".to_string(),
                Model::Gpt5p5 => "gpt-5.5".to_string(),
            }
        )
    }
}
