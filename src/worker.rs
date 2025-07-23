use std::env::VarError;

use crate::{nice_display::NiceDisplay, open_ai_key::OpenAiKey};

#[derive(Clone, Debug)]
pub struct Worker {
    pub open_ai_key: OpenAiKey,
    pub reqwest_client: reqwest::Client,
}

#[derive(Debug)]
pub enum InitError {
    OpenAiKey(VarError),
}

impl NiceDisplay for InitError {
    fn message(&self) -> String {
        match self {
            InitError::OpenAiKey(err) => format!("OpenAI API key error: {}", err),
        }
    }
}

impl Worker {
    pub fn new() -> Result<Self, InitError> {
        let open_ai_key = OpenAiKey::from_env().map_err(InitError::OpenAiKey)?;

        Ok(Worker {
            open_ai_key,
            reqwest_client: reqwest::Client::new(),
        })
    }
}
