use crate::nice_display::NiceDisplay;

use super::completion::CompletionError;

pub struct ToolCall {
    pub name: String,
    pub arguments: Vec<(String, serde_json::Value)>,
}
#[derive(Debug, Clone)]
pub enum ToolCallDecodeError {
    MissingField {
        field: String,
        json: serde_json::Value,
    },
    FieldWasNotArray {
        field: String,
        json: serde_json::Value,
    },
    FieldWasNotObject {
        field: String,
        json: serde_json::Value,
    },
    ArrayWasEmpty {
        which: String,
    },
    FieldWasNotString {
        field: String,
        json: serde_json::Value,
    },
    CouldParseString(String),
}

impl Into<CompletionError> for ToolCallDecodeError {
    fn into(self) -> CompletionError {
        CompletionError::ToolCallDecodeError(self)
    }
}

impl NiceDisplay for ToolCallDecodeError {
    fn message(&self) -> String {
        match self {
            ToolCallDecodeError::MissingField { field, json } => {
                format!("Missing field: {}\nHere is the json:\n {}", field, json)
            }
            ToolCallDecodeError::FieldWasNotArray { field, json } => {
                format!(
                    "Field was not an array: {}\nHere is the json:\n{}",
                    field, json
                )
            }
            ToolCallDecodeError::ArrayWasEmpty { which } => format!("Array was empty: {}", which),
            ToolCallDecodeError::FieldWasNotString { field, json } => {
                format!(
                    "Field was not a string: {}\nHere is the json:\n{}",
                    field, json
                )
            }
            ToolCallDecodeError::FieldWasNotObject { field, json } => {
                format!(
                    "Field was not an object: {}\nHere is the json:\n{}",
                    field, json
                )
            }
            ToolCallDecodeError::CouldParseString(err) => {
                format!("Could not parse string with serde:\n{}", err)
            }
        }
    }
}

impl ToolCall {
    pub fn from_json(json: &serde_json::Value) -> Result<Vec<Self>, ToolCallDecodeError> {
        let choices_json = json
            .get("choices")
            .ok_or_else(|| ToolCallDecodeError::MissingField {
                field: "choices".to_string(),
                json: json.clone(),
            })?
            .as_array()
            .ok_or_else(|| ToolCallDecodeError::FieldWasNotArray {
                field: "choices".to_string(),
                json: json.clone(),
            })?
            .first()
            .ok_or_else(|| ToolCallDecodeError::ArrayWasEmpty {
                which: "choices".to_string(),
            })?;

        let tool_call_jsons = choices_json
            .get("message")
            .ok_or_else(|| ToolCallDecodeError::MissingField {
                field: "message".to_string(),
                json: choices_json.clone(),
            })?
            .get("tool_calls")
            .ok_or_else(|| ToolCallDecodeError::MissingField {
                field: "tool_calls".to_string(),
                json: choices_json.clone(),
            })?
            .as_array()
            .ok_or_else(|| ToolCallDecodeError::FieldWasNotArray {
                field: "tool_calls".to_string(),
                json: choices_json.clone(),
            })?;

        tool_call_jsons
            .iter()
            .map(|tool_call_json| {
                let function_call_json = tool_call_json.get("function").ok_or_else(|| {
                    ToolCallDecodeError::MissingField {
                        field: "function".to_string(),
                        json: tool_call_json.clone(),
                    }
                })?;

                let name = function_call_json
                    .get("name")
                    .ok_or_else(|| ToolCallDecodeError::MissingField {
                        field: "name".to_string(),
                        json: function_call_json.clone(),
                    })?
                    .as_str()
                    .ok_or_else(|| ToolCallDecodeError::FieldWasNotString {
                        field: "name".to_string(),
                        json: function_call_json.clone(),
                    })?
                    .to_string();

                let arguments_string = function_call_json
                    .get("arguments")
                    .ok_or_else(|| ToolCallDecodeError::MissingField {
                        field: "arguments".to_string(),
                        json: function_call_json.clone(),
                    })?
                    .as_str()
                    .ok_or_else(|| ToolCallDecodeError::FieldWasNotString {
                        field: "arguments".to_string(),
                        json: function_call_json.clone(),
                    })?;

                let arguments: Vec<(String, serde_json::Value)> =
                    serde_json::from_str::<serde_json::Value>(arguments_string)
                        .map_err(|err| ToolCallDecodeError::CouldParseString(err.to_string()))?
                        .as_object()
                        .ok_or_else(|| ToolCallDecodeError::FieldWasNotObject {
                            field: "arguments".to_string(),
                            json: function_call_json.clone(),
                        })?
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<Vec<(String, serde_json::Value)>>();

                Ok(ToolCall { name, arguments })
            })
            .collect::<Result<Vec<ToolCall>, ToolCallDecodeError>>()
    }
}
