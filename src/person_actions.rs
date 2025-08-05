use crate::open_ai;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub enum PersonActionKind {
    Say,
}

impl PersonActionKind {
    pub fn to_open_ai_json_schema(&self) -> serde_json::Value {
        match self {
            PersonActionKind::Say => {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "comment": {
                            "type": "string",
                            "description": "The comment to say"
                        },
                        "recipients": {
                            "type": "array",
                            "items": {
                                "type": "string",
                            },
                            "description": "The recipients of the comment"
                        }
                    },
                    "required": ["comment", "recipients"],
                })
            }
        }
    }

    pub fn to_name(&self) -> String {
        match self {
            PersonActionKind::Say => "say".to_string(),
        }
    }
    pub fn to_open_ai_tool_function(&self) -> open_ai::ToolFunction {
        match self {
            PersonActionKind::Say => {
                let parameters = vec![
                    open_ai::ToolFunctionParameter::StringParam {
                        name: "comment".to_string(),
                        description: "The comment to say".to_string(),
                        required: true,
                    },
                    open_ai::ToolFunctionParameter::ArrayParam {
                        name: "recipients".to_string(),
                        description: "The recipients of the comment".to_string(),
                        item_type: open_ai::ArrayParamItemType::String,
                        required: true,
                    },
                ];

                open_ai::ToolFunction {
                    name: self.to_name(),
                    description: "Make the person say something to specified recipients"
                        .to_string(),
                    parameters,
                }
            }
        }
    }

    pub fn to_open_ai_tool(&self) -> open_ai::Tool {
        open_ai::Tool::FunctionCall(self.to_open_ai_tool_function())
    }
}

// pub enum PersonAction {
//     Say {
//         comment: String,
//         recipients: Vec<String>,
//     },
// }
//
// impl PersonAction {
//     pub fn say(comment: &str, recipients: Vec<String>) -> Self {
//         Self::Say {
//             comment: comment.to_string(),
//             recipients,
//         }
//     }
// }
