use crate::{nice_display::NiceDisplay, open_ai_key::OpenAiKey};

pub enum Model {
    Gpt4p1,
}

pub enum Role {
    Developer,
    User,
    Assistant,
}

impl Role {
    fn to_str(&self) -> &str {
        match self {
            Role::Developer => "developer",
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}

struct Message {
    role: Role,
    content: String,
}

impl Message {
    fn new(role: Role, content: &str) -> Self {
        Self {
            role,
            content: content.to_string(),
        }
    }
}

struct History {
    messages: Vec<Message>,
}

impl History {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    fn add_message(&mut self, role: Role, content: &str) {
        self.messages.push(Message::new(role, content));
    }
}

struct ChatResponse {
    created: u64,
    content: String,
}

pub struct Completion {
    model: String,
    history: History,
}

#[derive(Debug, Clone)]
pub enum CompletionError {
    RequestError(String),
    ResponseError(String),
    ResponseJsonDecodeError(String),
}

impl NiceDisplay for CompletionError {
    fn message(&self) -> String {
        match self {
            CompletionError::RequestError(err) => {
                format!("I had trouble making a request to open ai: {}", err)
            }
            CompletionError::ResponseError(err) => {
                format!("I had trouble with the response from open ai: {}", err)
            }
            CompletionError::ResponseJsonDecodeError(err) => {
                format!("I had trouble decoding the response from open ai: {}", err)
            }
        }
    }
}

impl Completion {
    pub fn new(model: Model) -> Self {
        let model_str = match model {
            Model::Gpt4p1 => "gpt-4o-2024-08-06",
        }
        .to_string();

        Self {
            model: model_str,
            history: History::new(),
        }
    }

    pub fn add_message(&mut self, role: Role, content: &str) -> &mut Self {
        self.history.add_message(role, content);
        self
    }

    pub async fn send_request(
        &self,
        open_ai_key: &OpenAiKey,
        client: reqwest::Client,
    ) -> Result<String, CompletionError> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": self.history.messages.iter().map(|msg| {
                serde_json::json!({
                    "role": msg.role.to_str(),
                    "content": msg.content,
                })
            }).collect::<Vec<_>>(),
        });

        let res = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Content-Type", "application/json")
            .header("Authorization", open_ai_key.to_header())
            .json(&body)
            .send()
            .await
            .map_err(|err| CompletionError::RequestError(err.to_string()))?
            .text()
            .await
            .map_err(|err| CompletionError::ResponseError(err.to_string()))?;

        let res_json: serde_json::Value = serde_json::from_str(&res)
            .map_err(|err| CompletionError::ResponseJsonDecodeError(err.to_string()))?;

        // let res = res_json["choices"][0]["message"]["content"];
        let message_res = res_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                CompletionError::ResponseError(format!("Missing content in response {}", res))
            })?
            .to_string();

        Ok(message_res)
    }
}
