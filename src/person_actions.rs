use crate::open_ai::completion::CompletionError;
use crate::open_ai::tool::{Tool, ToolFunction, ToolFunctionParameter};
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
            PersonActionKind::SayInScene => "say in scene".to_string(),
        }
    }

    pub fn all_action_names() -> Vec<String> {
        vec![
            PersonActionKind::Wait.to_name(),
            PersonActionKind::SayInScene.to_name(),
        ]
    }

    pub fn to_choice_tool() -> Tool {
        let parameters = vec![
            ToolFunctionParameter::StringEnumParam {
                name: "action".to_string(),
                description: "The single action to take.".to_string(),
                required: true,
                values: PersonActionKind::all_action_names(),
            },
            ToolFunctionParameter::StringParam {
                name: "comment".to_string(),
                description: "What to say if action is say in scene.".to_string(),
                required: false,
            },
            ToolFunctionParameter::IntegerParam {
                name: "duration".to_string(),
                description: "How long to wait in milliseconds if action is wait.".to_string(),
                required: false,
            },
        ];

        Tool::FunctionCall(ToolFunction::new(
            "choose_action".to_string(),
            "Choose a single action for the person. Only one action is allowed.".to_string(),
            parameters,
        ))
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
    UnrecognizedParameter {
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
            PersonActionError::UnrecognizedParameter {
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
        if tool_call_name.as_str() != "choose_action" {
            return Err(PersonActionError::UnrecognizedAction {
                action_name: tool_call_name,
            });
        }

        let mut maybe_action: Option<String> = None;
        let mut maybe_comment: Option<String> = None;
        let mut maybe_duration: Option<u64> = None;

        for (key, value) in tool_call.arguments {
            match key.as_str() {
                "action" => {
                    maybe_action = value.as_str().map(|s| s.to_string());
                }
                "comment" => {
                    maybe_comment = value.as_str().map(|s| s.to_string());
                }
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
                    Err(PersonActionError::UnrecognizedParameter {
                        action_name: tool_call_name.clone(),
                        parameter_name: key,
                    })?;
                }
            }
        }

        let action = maybe_action.ok_or_else(|| PersonActionError::ParameterMissing {
            action_name: tool_call_name.clone(),
            parameter_name: "action".to_string(),
        })?;

        match action.as_str() {
            "say in scene" => {
                let comment = maybe_comment.ok_or_else(|| PersonActionError::ParameterMissing {
                    action_name: tool_call_name.clone(),
                    parameter_name: "comment".to_string(),
                })?;
                Ok(PersonAction::SayInScene { comment })
            }
            "wait" => {
                let duration =
                    maybe_duration.ok_or_else(|| PersonActionError::ParameterMissing {
                        action_name: tool_call_name.clone(),
                        parameter_name: "duration".to_string(),
                    })?;
                Ok(PersonAction::Wait { duration })
            }
            _ => Err(PersonActionError::UnrecognizedAction {
                action_name: action,
            }),
        }
    }
}
