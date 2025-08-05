use crate::open_ai;
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
            PersonActionKind::Wait => {
                let parameters = vec![open_ai::ToolFunctionParameter::IntegerParam {
                    name: "duration".to_string(),
                    description: "The duration to wait in millisoconds".to_string(),
                    required: true,
                }];

                open_ai::ToolFunction {
                    name: self.to_name(),
                    description: "Make the person wait for a specified duration".to_string(),
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
