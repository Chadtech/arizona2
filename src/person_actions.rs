use crate::open_ai::completion::CompletionError;
use crate::open_ai::tool::ToolFunctionParameter;
use crate::open_ai::tool_call::ToolCall;
use crate::{nice_display::NiceDisplay, open_ai};

pub enum PersonActionKind {
    Wait,
    SayInScene,
}

impl PersonActionKind {
    pub fn to_name(&self) -> String {
        match self {
            PersonActionKind::Wait => "wait".to_string(),
            PersonActionKind::SayInScene => "say_in_scene".to_string(),
        }
    }
    pub fn to_open_ai_tool_function(&self) -> open_ai::tool::ToolFunction {
        match self {
            PersonActionKind::SayInScene => {
                let parameters = vec![ToolFunctionParameter::StringParam {
                    name: "comment".to_string(),
                    description: "The comment to say".to_string(),
                    required: true,
                }];

                open_ai::tool::ToolFunction {
                    name: self.to_name(),
                    description: "Make the person say something in the scene they are in, which will be heard by everyone in the scene"
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

    pub fn all() -> Vec<PersonActionKind> {
        vec![PersonActionKind::Wait, PersonActionKind::SayInScene]
    }
}

#[derive(Debug, Clone)]
pub enum PersonAction {
    Wait { duration: u64 },
    SayInScene { comment: String },
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
            "say_in_scene" => {
                let mut maybe_comment = None;

                for (key, value) in tool_call.arguments {
                    match key.as_str() {
                        "comment" => {
                            maybe_comment = value.as_str().map(|s| s.to_string());
                        }
                        _ => {
                            Err(PersonActionError::UnrecongizedParameter {
                                action_name: tool_call_name.clone(),
                                parameter_name: key,
                            })?;
                        }
                    }
                }

                match maybe_comment {
                    Some(comment) => Ok(PersonAction::SayInScene { comment }),
                    None => Err(PersonActionError::ParameterMissing {
                        action_name: tool_call_name.clone(),
                        parameter_name: "comment".to_string(),
                    }),
                }
            }
            "wait" => {
                let mut maybe_duration: Option<u64> = None;

                for (key, value) in tool_call.arguments {
                    match key.as_str() {
                        "duration" => {
                            if let Some(dur) = value.as_u64() {
                                maybe_duration = Some(dur);
                            } else {
                                Err(PersonActionError::UnexpectedType {
                                    action_name: tool_call_name.clone(),
                                    parameter_name: "duration".to_string(),
                                    wanted_type: "u64".to_string(),
                                })?
                            }
                        }
                        _ => {
                            Err(PersonActionError::UnrecongizedParameter {
                                action_name: tool_call_name.clone(),
                                parameter_name: key,
                            })?;
                        }
                    }
                }

                let duration =
                    maybe_duration.ok_or_else(|| PersonActionError::ParameterMissing {
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
