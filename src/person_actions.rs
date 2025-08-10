use crate::open_ai::completion::CompletionError;
use crate::open_ai::tool::ToolFunctionParameter;
use crate::open_ai::tool_call::ToolCall;
use crate::{nice_display::NiceDisplay, open_ai};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub enum PersonActionKind {
    Say,
    Wait,
}

impl PersonActionKind {
    pub fn to_name(&self) -> String {
        match self {
            PersonActionKind::Say => "say".to_string(),
            PersonActionKind::Wait => "wait".to_string(),
        }
    }
    pub fn to_open_ai_tool_function(&self) -> open_ai::tool::ToolFunction {
        match self {
            PersonActionKind::Say => {
                let parameters = vec![
                    open_ai::tool::ToolFunctionParameter::StringParam {
                        name: "comment".to_string(),
                        description: "The comment to say".to_string(),
                        required: true,
                    },
                    open_ai::tool::ToolFunctionParameter::ArrayParam {
                        name: "recipients".to_string(),
                        description: "The recipients of the comment".to_string(),
                        item_type: open_ai::tool::ArrayParamItemType::String,
                        required: true,
                    },
                ];

                open_ai::tool::ToolFunction {
                    name: self.to_name(),
                    description: "Make the person say something to specified recipients"
                        .to_string(),
                    parameters,
                }
            }
            PersonActionKind::Wait => {
                let parameters = vec![ToolFunctionParameter::IntegerParam {
                    name: "duration".to_string(),
                    description: "The duration to wait in millisoconds".to_string(),
                    required: true,
                }];

                open_ai::tool::ToolFunction {
                    name: self.to_name(),
                    description: "Make the person wait for a specified duration".to_string(),
                    parameters,
                }
            }
        }
    }

    pub fn to_open_ai_tool(&self) -> open_ai::tool::Tool {
        open_ai::tool::Tool::FunctionCall(self.to_open_ai_tool_function())
    }
}

#[derive(Debug, Clone)]
pub enum PersonAction {
    Say {
        comment: String,
        recipients: Vec<String>,
    },
    Wait {
        duration: u64,
    },
}

impl Into<CompletionError> for PersonActionError {
    fn into(self) -> CompletionError {
        CompletionError::PersonActionError(self)
    }
}

#[derive(Debug, Clone)]
pub enum PersonActionError {
    UnrecognizedAction {
        action_name: String,
    },
    UnrecongizedParameter {
        action_name: String,
        parameter_name: String,
    },
    ParameterMissing {
        action_name: String,
        parameter_name: String,
    },
    UnexpectedType {
        action_name: String,
        parameter_name: String,
        wanted_type: String,
    },
}

impl NiceDisplay for PersonActionError {
    fn message(&self) -> String {
        match self {
            PersonActionError::UnrecognizedAction { action_name } => {
                format!("Unrecognized action: {}", action_name)
            }
            PersonActionError::UnrecongizedParameter {
                action_name,
                parameter_name,
            } => format!(
                "Unrecognized parameter '{}' for action '{}'",
                parameter_name, action_name
            ),
            PersonActionError::ParameterMissing {
                action_name,
                parameter_name,
            } => format!(
                "Missing required parameter '{}' for action '{}'",
                parameter_name, action_name
            ),
            PersonActionError::UnexpectedType {
                action_name,
                parameter_name,
                wanted_type,
            } => format!(
                "Unexpected type for parameter '{}' in action '{}'. Expected type: {}",
                parameter_name, action_name, wanted_type
            ),
        }
    }
}

impl PersonAction {
    pub fn from_open_ai_tool_call(tool_call: ToolCall) -> Result<Self, PersonActionError> {
        let tool_call_name = tool_call.name;
        match tool_call_name.as_str() {
            "say" => {
                let mut comment = None;
                let mut recipients = None;

                for (key, value) in tool_call.arguments {
                    match key.as_str() {
                        "comment" => {
                            comment = value.as_str().map(|s| s.to_string());
                        }
                        "recipients" => {
                            recipients = value.as_array().map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .collect()
                            });
                        }
                        _ => {
                            Err(PersonActionError::UnrecongizedParameter {
                                action_name: tool_call_name.clone(),
                                parameter_name: key,
                            })?;
                        }
                    }
                }

                match (comment, recipients) {
                    (Some(comment), Some(recipients)) => Ok(PersonAction::Say {
                        comment,
                        recipients,
                    }),
                    (None, _) => Err(PersonActionError::ParameterMissing {
                        action_name: tool_call_name.clone(),
                        parameter_name: "comment".to_string(),
                    }),
                    (_, None) => Err(PersonActionError::ParameterMissing {
                        action_name: tool_call_name.clone(),
                        parameter_name: "recipients".to_string(),
                    }),
                }
            }
            "wait" => {
                let duration: u64 = tool_call
                    .arguments
                    .into_iter()
                    .map(|(key, value)| {
                        if key == "duration" {
                            value
                                .as_u64()
                                .ok_or_else(|| PersonActionError::UnexpectedType {
                                    action_name: tool_call_name.clone(),
                                    parameter_name: "duration".to_string(),
                                    wanted_type: "u64".to_string(),
                                })
                        } else {
                            Err(PersonActionError::UnrecongizedParameter {
                                action_name: tool_call_name.clone(),
                                parameter_name: key,
                            })
                        }
                    })
                    .collect::<Result<Vec<u64>, PersonActionError>>()?
                    .first()
                    .cloned()
                    .ok_or_else(|| PersonActionError::ParameterMissing {
                        action_name: tool_call_name.clone(),
                        parameter_name: "duration".to_string(),
                    })?;

                Ok(PersonAction::Wait { duration })
            }
            _ => Err(PersonActionError::UnrecognizedAction {
                action_name: tool_call_name,
            }),
        }
    }
}
