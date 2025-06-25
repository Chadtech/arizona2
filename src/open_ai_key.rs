use std::env::VarError;

#[derive(Clone)]
pub struct OpenAiKey {
    key: String,
}

impl OpenAiKey {
    pub fn from_env() -> Result<Self, VarError> {
        std::env::var("OPEN_AI_API_KEY").map(|key| OpenAiKey { key })
    }

    pub fn to_header(&self) -> String {
        format!("Bearer {}", self.key)
    }
}
