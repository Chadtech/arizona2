use crate::nice_display::NiceDisplay;
use crate::open_ai::history::History;
use crate::open_ai::model::Model;
use crate::open_ai::role::Role;
use crate::open_ai::tool::Tool;
use crate::open_ai::tool_call;
use crate::open_ai::tool_call::ToolCall;
use crate::open_ai_key::OpenAiKey;
use crate::person_actions::PersonActionError;

pub struct Completion {
    model: String,
    history: History,
    tool_call: Vec<Tool>,
}

pub struct Response {
    json: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum MessageError {
    MissingField {
        field: String,
        json: serde_json::Value,
    },
    NoChoices {
        json: serde_json::Value,
    },
    NotString {
        what: String,
        json: serde_json::Value,
    },
}

impl From<MessageError> for CompletionError {
    fn from(val: MessageError) -> Self {
        CompletionError::Message(val)
    }
}

impl NiceDisplay for MessageError {
    fn message(&self) -> String {
        match self {
            MessageError::MissingField { field, json } => {
                format!(
                    "Missing field: {}\nResponse JSON:\n{}",
                    field,
                    format_json(json)
                )
            }
            MessageError::NoChoices { json } => {
                format!(
                    "No choices in response\nResponse JSON:\n{}",
                    format_json(json)
                )
            }
            MessageError::NotString { what, json } => {
                format!(
                    "Field is not a string: {}\nResponse JSON:\n{}",
                    what,
                    format_json(json)
                )
            }
        }
    }
}

impl Response {
    fn new(json: serde_json::Value) -> Self {
        Self { json }
    }

    pub fn as_pretty_json(&self) -> String {
        serde_json::to_string_pretty(&self.json).unwrap_or_else(|_| self.json.to_string())
    }

    pub fn as_message(&self) -> Result<String, MessageError> {
        self.json
            .get("choices")
            .ok_or_else(|| MessageError::MissingField {
                field: "choices".to_string(),
                json: self.json.clone(),
            })?
            .get(0)
            .ok_or_else(|| MessageError::NoChoices {
                json: self.json.clone(),
            })?
            .get("message")
            .ok_or_else(|| MessageError::MissingField {
                field: "message".to_string(),
                json: self.json.clone(),
            })?
            .get("content")
            .ok_or_else(|| MessageError::MissingField {
                field: "content".to_string(),
                json: self.json.clone(),
            })?
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| MessageError::NotString {
                what: "content".to_string(),
                json: self.json.clone(),
            })
    }

    pub fn as_tool_calls(&self) -> Result<Vec<ToolCall>, tool_call::ToolCallDecodeError> {
        ToolCall::from_json(&self.json)
    }

    pub fn maybe_tool_calls(
        &self,
    ) -> Result<Option<Vec<ToolCall>>, tool_call::ToolCallDecodeError> {
        let choices_json = self
            .json
            .get("choices")
            .ok_or_else(|| tool_call::ToolCallDecodeError::MissingField {
                field: "choices".to_string(),
                json: self.json.clone(),
            })?
            .as_array()
            .ok_or_else(|| tool_call::ToolCallDecodeError::FieldWasNotArray {
                field: "choices".to_string(),
                json: self.json.clone(),
            })?
            .first()
            .ok_or_else(|| tool_call::ToolCallDecodeError::ArrayWasEmpty {
                which: "choices".to_string(),
            })?;

        let message = choices_json.get("message").ok_or_else(|| {
            tool_call::ToolCallDecodeError::MissingField {
                field: "message".to_string(),
                json: choices_json.clone(),
            }
        })?;

        let tool_calls_value = message.get("tool_calls");
        match tool_calls_value {
            None => Ok(None),
            Some(value) if value.is_null() => Ok(None),
            Some(value) => {
                value.as_array().ok_or_else(|| {
                    tool_call::ToolCallDecodeError::FieldWasNotArray {
                        field: "tool_calls".to_string(),
                        json: message.clone(),
                    }
                })?;
                ToolCall::from_json(&self.json).map(Some)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum CompletionError {
    Request(String),
    Response(String),
    ResponseJsonDecode(String),
    Message(MessageError),
    ToolCallDecode(tool_call::ToolCallDecodeError),
    PersonAction(PersonActionError),
}

impl NiceDisplay for CompletionError {
    fn message(&self) -> String {
        match self {
            CompletionError::Request(err) => {
                format!("I had trouble making a request to open ai: {}", err)
            }
            CompletionError::Response(err) => {
                format!("I had trouble with the response from open ai: {}", err)
            }
            CompletionError::ResponseJsonDecode(err) => {
                format!("I had trouble decoding the response from open ai: {}", err)
            }
            CompletionError::Message(err) => {
                format!(
                    "I had trouble extracting the message from the response:\n{:?}",
                    err.message()
                )
            }
            CompletionError::ToolCallDecode(err) => {
                format!(
                    "I had trouble decoding the tool calls from the response:\n{}",
                    err.message()
                )
            }
            CompletionError::PersonAction(err) => {
                format!("I had trouble interpreting the action: {}", err.message())
            }
        }
    }
}

fn format_json(value: &serde_json::Value) -> String {
    match serde_json::to_string_pretty(value) {
        Ok(text) => text,
        Err(_) => value.to_string(),
    }
}

impl Completion {
    pub fn new(model: Model) -> Self {
        let model_str = model.to_string();

        Self {
            model: model_str,
            history: History::new(),
            tool_call: vec![],
        }
    }

    pub fn add_message(&mut self, role: Role, content: &str) -> &mut Self {
        self.history.add_message(role, content);
        self
    }

    pub fn add_tool_call(&mut self, tool: Tool) -> &mut Self {
        self.tool_call.push(tool);
        self
    }

    pub async fn send_request(
        &self,
        open_ai_key: &OpenAiKey,
        client: reqwest::Client,
    ) -> Result<Response, CompletionError> {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": self.history.get_messages().iter().map(|msg| {
                serde_json::json!({
                    "role": msg.role().to_str(),
                    "content": msg.content(),
                })
            }).collect::<Vec<_>>()
        });

        if !self.tool_call.is_empty() {
            body["tools"] = self
                .tool_call
                .iter()
                .map(|tool| tool.to_json())
                .collect::<serde_json::Value>();
            body["parallel_tool_calls"] = serde_json::json!(false);
        }

        let res = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Content-Type", "application/json")
            .header("Authorization", open_ai_key.to_header())
            .json(&body)
            .send()
            .await
            .map_err(|err| CompletionError::Request(err.to_string()))?
            .text()
            .await
            .map_err(|err| CompletionError::Response(err.to_string()))?;

        let res_json: serde_json::Value = serde_json::from_str(&res)
            .map_err(|err| CompletionError::ResponseJsonDecode(err.to_string()))?;

        Ok(Response::new(res_json))
    }
}
