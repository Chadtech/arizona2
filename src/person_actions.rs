use crate::nice_display::NiceDisplay;
use crate::open_ai::completion::CompletionError;
use crate::open_ai::tool::{Tool, ToolFunction, ToolFunctionParameter};
use crate::open_ai::tool_call::ToolCall;

pub enum PersonActionKind {
    Wait,
    Hibernate,
    Idle,
    GazeInScene,
    SayInScene,
    MoveToScene,
}

#[derive(Debug, Clone)]
pub enum ReflectionDecision {
    Reflection,
    NoReflection,
}

impl ReflectionDecision {
    pub fn to_name(&self) -> String {
        match self {
            ReflectionDecision::Reflection => "reflection".to_string(),
            ReflectionDecision::NoReflection => "no_reflection".to_string(),
        }
    }

    pub fn all_names() -> Vec<String> {
        vec![
            ReflectionDecision::NoReflection.to_name(),
            ReflectionDecision::Reflection.to_name(),
        ]
    }

    pub fn from_optional_tool_value(value: Option<String>) -> Result<Self, PersonActionError> {
        match value {
            Some(name) => match name.as_str() {
                "reflection" => Ok(ReflectionDecision::Reflection),
                "no_reflection" => Ok(ReflectionDecision::NoReflection),
                _ => Err(PersonActionError::UnrecognizedReflection {
                    reflection_name: name,
                }),
            },
            None => Ok(ReflectionDecision::NoReflection),
        }
    }
}

impl PersonActionKind {
    pub fn to_name(&self) -> String {
        match self {
            PersonActionKind::Wait => "wait".to_string(),
            PersonActionKind::Hibernate => "hibernate".to_string(),
            PersonActionKind::Idle => "idle".to_string(),
            PersonActionKind::GazeInScene => "gaze in scene".to_string(),
            PersonActionKind::SayInScene => "say in scene".to_string(),
            PersonActionKind::MoveToScene => "move to scene".to_string(),
        }
    }

    pub fn all_action_names() -> Vec<String> {
        vec![
            PersonActionKind::Wait.to_name(),
            PersonActionKind::Hibernate.to_name(),
            PersonActionKind::Idle.to_name(),
            PersonActionKind::GazeInScene.to_name(),
            PersonActionKind::SayInScene.to_name(),
            PersonActionKind::MoveToScene.to_name(),
        ]
    }

    pub fn to_choice_tool() -> Tool {
        let parameters = vec![
            ToolFunctionParameter::StringEnum {
                name: "reflection".to_string(),
                description: "Whether the person should reflect after acting.".to_string(),
                required: false,
                values: ReflectionDecision::all_names(),
            },
            ToolFunctionParameter::StringEnum {
                name: "action".to_string(),
                description: "The single action to take.".to_string(),
                required: true,
                values: PersonActionKind::all_action_names(),
            },
            ToolFunctionParameter::String {
                name: "comment".to_string(),
                description: "What to say if action is say in scene. Write like spoken dialogue, not a document: avoid bullet points, numbered lists, headings, and list-like enumeration."
                    .to_string(),
                required: false,
            },
            ToolFunctionParameter::String {
                name: "destination_scene_name".to_string(),
                description:
                    "Optional destination scene if action is say in scene and the person should leave immediately after speaking."
                        .to_string(),
                required: false,
            },
            ToolFunctionParameter::String {
                name: "scene_name".to_string(),
                description: "Scene name to move to if action is move to scene.".to_string(),
                required: false,
            },
            ToolFunctionParameter::Integer {
                name: "duration".to_string(),
                description:
                    "How long to wait or hibernate in milliseconds if action is wait or hibernate."
                        .to_string(),
                required: false,
            },
        ];

        Tool::FunctionCall(ToolFunction::new(
            "choose_action".to_string(),
            "Choose a single action for the person. Only one action is allowed. Use idle when the person decides to do nothing. Use hibernate for long, uninterrupted sleep. If action is say in scene, the comment should resemble natural speech rather than a document or list. You may also provide destination_scene_name to leave right after speaking."
                .to_string(),
            parameters,
        ))
    }
}

#[derive(Debug, Clone)]
pub enum PersonAction {
    Wait {
        duration: u64,
    },
    Hibernate {
        duration: u64,
    },
    Idle,
    GazeInScene,
    SayInScene {
        comment: String,
        destination_scene_name: Option<String>,
    },
    MoveToScene {
        scene_name: String,
    },
}

impl PersonAction {
    pub fn summarize(&self) -> String {
        match self {
            PersonAction::Wait { duration } => {
                format!("Waited for {} seconds.", duration)
            }
            PersonAction::Hibernate { duration } => {
                format!("Hibernated for {} seconds.", duration)
            }
            PersonAction::Idle => "Did nothing.".to_string(),
            PersonAction::GazeInScene => "Gazed around the current scene.".to_string(),
            PersonAction::SayInScene {
                comment,
                destination_scene_name,
            } => match destination_scene_name {
                Some(scene_name) => {
                    format!("Spoke in scene then left for {}: {}", scene_name, comment)
                }
                None => format!("Spoke in scene: {}", comment),
            },
            PersonAction::MoveToScene { scene_name } => {
                format!("Moved to scene: {}", scene_name)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PersonReaction {
    pub action: PersonAction,
    pub reflection: ReflectionDecision,
}

impl From<PersonActionError> for CompletionError {
    fn from(val: PersonActionError) -> Self {
        CompletionError::PersonAction(val)
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
        arguments: serde_json::Value,
    },
    UnexpectedType {
        action_name: String,
        parameter_name: String,
        wanted_type: String,
    },
    UnrecognizedReflection {
        reflection_name: String,
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
                arguments,
            } => format!(
                "Missing required parameter '{}' for action '{}'. Arguments: {}",
                parameter_name, action_name, arguments
            ),
            PersonActionError::UnexpectedType {
                action_name,
                parameter_name,
                wanted_type,
            } => format!(
                "Unexpected type for parameter '{}' in action '{}'. Expected type: {}",
                parameter_name, action_name, wanted_type
            ),
            PersonActionError::UnrecognizedReflection { reflection_name } => {
                format!("Unrecognized reflection value: {}", reflection_name)
            }
        }
    }
}

impl PersonReaction {
    pub fn from_open_ai_tool_call(tool_call: ToolCall) -> Result<Self, PersonActionError> {
        let tool_call_name = tool_call.name;
        if tool_call_name.as_str() != "choose_action" {
            return Err(PersonActionError::UnrecognizedAction {
                action_name: tool_call_name,
            });
        }

        let arguments = tool_call.arguments;
        let arguments_json = tool_args_to_json(&arguments);

        let mut maybe_reflection: Option<String> = None;
        let mut maybe_action: Option<String> = None;
        let mut maybe_comment: Option<String> = None;
        let mut maybe_destination_scene_name: Option<String> = None;
        let mut maybe_scene_name: Option<String> = None;
        let mut maybe_duration: Option<u64> = None;

        for (key, value) in arguments {
            match key.as_str() {
                "reflection" => {
                    maybe_reflection = normalized_non_empty_string(&value);
                }
                "action" => {
                    maybe_action = normalized_non_empty_string(&value);
                }
                "comment" => {
                    maybe_comment = normalized_non_empty_string(&value);
                }
                "destination_scene_name" => {
                    maybe_destination_scene_name = normalized_non_empty_string(&value);
                }
                "scene_name" => {
                    maybe_scene_name = normalized_non_empty_string(&value);
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

        let reflection = ReflectionDecision::from_optional_tool_value(maybe_reflection)?;

        let action = maybe_action.ok_or_else(|| PersonActionError::ParameterMissing {
            action_name: tool_call_name.clone(),
            parameter_name: "action".to_string(),
            arguments: arguments_json.clone(),
        })?;

        let action = match action.as_str() {
            "say in scene" => {
                let comment = maybe_comment.ok_or_else(|| PersonActionError::ParameterMissing {
                    action_name: tool_call_name.clone(),
                    parameter_name: "comment".to_string(),
                    arguments: arguments_json.clone(),
                })?;
                PersonAction::SayInScene {
                    comment,
                    destination_scene_name: maybe_destination_scene_name,
                }
            }
            "wait" => {
                let duration =
                    maybe_duration.ok_or_else(|| PersonActionError::ParameterMissing {
                        action_name: tool_call_name.clone(),
                        parameter_name: "duration".to_string(),
                        arguments: arguments_json.clone(),
                    })?;
                PersonAction::Wait { duration }
            }
            "hibernate" => {
                let duration =
                    maybe_duration.ok_or_else(|| PersonActionError::ParameterMissing {
                        action_name: tool_call_name.clone(),
                        parameter_name: "duration".to_string(),
                        arguments: arguments_json.clone(),
                    })?;
                PersonAction::Hibernate { duration }
            }
            "idle" => PersonAction::Idle,
            "gaze in scene" => PersonAction::GazeInScene,
            "move to scene" => {
                let scene_name =
                    maybe_scene_name.ok_or_else(|| PersonActionError::ParameterMissing {
                        action_name: tool_call_name.clone(),
                        parameter_name: "scene_name".to_string(),
                        arguments: arguments_json.clone(),
                    })?;
                PersonAction::MoveToScene { scene_name }
            }
            _ => Err(PersonActionError::UnrecognizedAction {
                action_name: action,
            })?,
        };

        Ok(PersonReaction { action, reflection })
    }
}

fn tool_args_to_json(args: &[(String, serde_json::Value)]) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (key, value) in args {
        map.insert(key.clone(), value.clone());
    }
    serde_json::Value::Object(map)
}

fn normalized_non_empty_string(value: &serde_json::Value) -> Option<String> {
    let trimmed = value.as_str()?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn choose_action_call(arguments: Vec<(String, serde_json::Value)>) -> ToolCall {
        ToolCall {
            name: "choose_action".to_string(),
            arguments,
        }
    }

    #[test]
    fn test_blank_destination_scene_name_becomes_none() {
        let reaction = PersonReaction::from_open_ai_tool_call(choose_action_call(vec![
            ("reflection".to_string(), json!("no_reflection")),
            ("action".to_string(), json!("say in scene")),
            ("comment".to_string(), json!("hello there")),
            ("destination_scene_name".to_string(), json!("   ")),
        ]))
        .unwrap();

        match reaction.action {
            PersonAction::SayInScene {
                destination_scene_name,
                ..
            } => {
                assert_eq!(destination_scene_name, None);
            }
            other => panic!("unexpected action: {:?}", other),
        }
    }

    #[test]
    fn test_blank_move_scene_name_is_rejected() {
        let err = PersonReaction::from_open_ai_tool_call(choose_action_call(vec![
            ("reflection".to_string(), json!("no_reflection")),
            ("action".to_string(), json!("move to scene")),
            ("scene_name".to_string(), json!("   ")),
        ]))
        .unwrap_err();

        match err {
            PersonActionError::ParameterMissing { parameter_name, .. } => {
                assert_eq!(parameter_name, "scene_name");
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn test_scene_name_is_trimmed() {
        let reaction = PersonReaction::from_open_ai_tool_call(choose_action_call(vec![
            ("reflection".to_string(), json!("no_reflection")),
            ("action".to_string(), json!("move to scene")),
            ("scene_name".to_string(), json!("  kitchen  ")),
        ]))
        .unwrap();

        match reaction.action {
            PersonAction::MoveToScene { scene_name } => {
                assert_eq!(scene_name, "kitchen");
            }
            other => panic!("unexpected action: {:?}", other),
        }
    }

    #[test]
    fn test_missing_reflection_defaults_to_no_reflection() {
        let reaction = PersonReaction::from_open_ai_tool_call(choose_action_call(vec![(
            "action".to_string(),
            json!("idle"),
        )]))
        .unwrap();

        match reaction.reflection {
            ReflectionDecision::NoReflection => {}
            ReflectionDecision::Reflection => {
                panic!("unexpected reflection value");
            }
        }
    }

    #[test]
    fn test_unrecognized_reflection_is_rejected() {
        let err = PersonReaction::from_open_ai_tool_call(choose_action_call(vec![
            ("reflection".to_string(), json!("maybe")),
            ("action".to_string(), json!("idle")),
        ]))
        .unwrap_err();

        match err {
            PersonActionError::UnrecognizedReflection { reflection_name } => {
                assert_eq!(reflection_name, "maybe");
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn test_gaze_in_scene_action_parses() {
        let reaction = PersonReaction::from_open_ai_tool_call(choose_action_call(vec![(
            "action".to_string(),
            json!("gaze in scene"),
        )]))
        .unwrap();

        match reaction.action {
            PersonAction::GazeInScene => {}
            other => panic!("unexpected action: {:?}", other),
        }
    }
}
