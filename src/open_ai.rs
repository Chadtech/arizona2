use crate::{nice_display::NiceDisplay, open_ai_key::OpenAiKey};

pub enum Model {
    Gpt4p1,
}

pub enum Role {
    System,
    User,
    Assistant,
}

impl Role {
    fn to_str(&self) -> &str {
        match self {
            Role::System => "system",
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

pub enum Tool {
    FunctionCall(ToolFunction),
}

pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolFunctionParameter>,
}

impl ToolFunction {
    pub fn new(name: String, description: String, parameters: Vec<ToolFunctionParameter>) -> Self {
        Self {
            name,
            description,
            parameters,
        }
    }
}

impl Into<Tool> for ToolFunction {
    fn into(self) -> Tool {
        Tool::FunctionCall(self)
    }
}

impl Tool {
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Tool::FunctionCall(func) => {
                let mut properties = serde_json::json!({});

                for param in &func.parameters {
                    match param {
                        ToolFunctionParameter::StringParam {
                            name,
                            description,
                            required,
                        } => {
                            properties[name] = serde_json::json!({
                                "type": "string",
                                "description": description,
                            });
                        }
                        ToolFunctionParameter::ArrayParam {
                            name,
                            description,
                            item_type,
                            required,
                        } => {
                            let item_type_str = match item_type {
                                ArrayParamItemType::String => "string",
                            };
                            properties[name] = serde_json::json!({
                                "type": "array",
                                "items": { "type": item_type_str },
                                "description": description,
                            });
                        }
                        ToolFunctionParameter::IntegerParam {
                            name,
                            description,
                            required,
                        } => {
                            properties[name] = serde_json::json!({
                                "type": "integer",
                                "description": description,
                            });
                        }
                    }
                }

                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": func.name,
                        "description": func.description,
                        "parameters": serde_json::json!({
                            "type": "object",
                            "properties": properties,
                            "required": func.parameters.iter().filter_map(|param| {
                                if param.required() {
                                    Some(param.name())
                                } else {
                                    None
                                }
                            }).collect::<Vec<_>>(),
                        }),
                    },
                })
            }
        }
    }
}

pub enum ToolFunctionParameter {
    StringParam {
        name: String,
        description: String,
        required: bool,
    },
    ArrayParam {
        name: String,
        description: String,
        item_type: ArrayParamItemType,
        required: bool,
    },
    IntegerParam {
        name: String,
        description: String,
        required: bool,
    },
}

impl ToolFunctionParameter {
    pub fn required(&self) -> bool {
        match *self {
            ToolFunctionParameter::StringParam { required, .. } => required,
            ToolFunctionParameter::ArrayParam { required, .. } => required,
            ToolFunctionParameter::IntegerParam { required, .. } => required,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            ToolFunctionParameter::StringParam { name, .. } => name,
            ToolFunctionParameter::ArrayParam { name, .. } => name,
            ToolFunctionParameter::IntegerParam { name, .. } => name,
        }
    }
}

pub enum ArrayParamItemType {
    String,
}

pub struct Completion {
    model: String,
    history: History,
    tool_call: Vec<Tool>,
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
    ) -> Result<String, CompletionError> {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": self.history.messages.iter().map(|msg| {
                serde_json::json!({
                    "role": msg.role.to_str(),
                    "content": msg.content,
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
