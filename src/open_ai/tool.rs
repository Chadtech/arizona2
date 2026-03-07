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

pub enum ToolFunctionParameter {
    String {
        name: String,
        description: String,
        required: bool,
    },
    StringEnum {
        name: String,
        description: String,
        required: bool,
        values: Vec<String>,
    },
    Integer {
        name: String,
        description: String,
        required: bool,
    },
}

impl ToolFunctionParameter {
    pub fn required(&self) -> bool {
        match *self {
            ToolFunctionParameter::String { required, .. } => required,
            ToolFunctionParameter::StringEnum { required, .. } => required,
            ToolFunctionParameter::Integer { required, .. } => required,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            ToolFunctionParameter::String { name, .. } => name,
            ToolFunctionParameter::StringEnum { name, .. } => name,
            ToolFunctionParameter::Integer { name, .. } => name,
        }
    }
}

impl From<ToolFunction> for Tool {
    fn from(val: ToolFunction) -> Self {
        Tool::FunctionCall(val)
    }
}

impl Tool {
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Tool::FunctionCall(func) => {
                let mut properties = serde_json::json!({});

                for param in &func.parameters {
                    match param {
                        ToolFunctionParameter::String {
                            name,
                            description,
                            required: _,
                        } => {
                            properties[name] = serde_json::json!({
                                "type": "string",
                                "description": description,
                            });
                        }
                        ToolFunctionParameter::StringEnum {
                            name,
                            description,
                            values,
                            required: _,
                        } => {
                            properties[name] = serde_json::json!({
                                "type": "string",
                                "description": description,
                                "enum": values,
                            });
                        }
                        ToolFunctionParameter::Integer {
                            name,
                            description,
                            required: _,
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
