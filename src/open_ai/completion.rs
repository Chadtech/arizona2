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
    MissingField(String),
    NoChoices,
    NotString { what: String },
}

impl Into<CompletionError> for MessageError {
    fn into(self) -> CompletionError {
        CompletionError::MessageError(self)
    }
}

impl NiceDisplay for MessageError {
    fn message(&self) -> String {
        match self {
            MessageError::MissingField(field) => format!("Missing field: {}", field),
            MessageError::NoChoices => "No choices in response".to_string(),
            MessageError::NotString { what } => format!("Field is not a string: {}", what),
        }
    }
}

impl Response {
    fn new(json: serde_json::Value) -> Self {
        Self { json }
    }

    pub fn as_message(&self) -> Result<String, MessageError> {
        self.json
            .get("choices")
            .ok_or_else(|| MessageError::MissingField("choices".to_string()))?
            .get(0)
            .ok_or_else(|| MessageError::NoChoices)?
            .get("message")
            .ok_or_else(|| MessageError::MissingField("message".to_string()))?
            .get("content")
            .ok_or_else(|| MessageError::MissingField("content".to_string()))?
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| MessageError::NotString {
                what: "content".to_string(),
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

        let message = choices_json
            .get("message")
            .ok_or_else(|| tool_call::ToolCallDecodeError::MissingField {
                field: "message".to_string(),
                json: choices_json.clone(),
            })?;

        let tool_calls_value = message.get("tool_calls");
        match tool_calls_value {
            None => Ok(None),
            Some(value) if value.is_null() => Ok(None),
            Some(value) => {
                value
                    .as_array()
                    .ok_or_else(|| tool_call::ToolCallDecodeError::FieldWasNotArray {
                        field: "tool_calls".to_string(),
                        json: message.clone(),
                    })?;
                ToolCall::from_json(&self.json).map(Some)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum CompletionError {
    RequestError(String),
    ResponseError(String),
    ResponseJsonDecodeError(String),
    MessageError(MessageError),
    ToolCallDecodeError(tool_call::ToolCallDecodeError),
    PersonActionError(PersonActionError),
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
            CompletionError::MessageError(err) => {
                format!(
                    "I had trouble extracting the message from the response:\n{:?}",
                    err.message()
                )
            }
            CompletionError::ToolCallDecodeError(err) => {
                format!(
                    "I had trouble decoding the tool calls from the response:\n{}",
                    err.message()
                )
            }
            CompletionError::PersonActionError(err) => {
                format!("I had trouble interpreting the action: {}", err.message())
            }
        }
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
        }

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

        Ok(Response::new(res_json))
    }
}
