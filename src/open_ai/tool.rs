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
    StringParam {
        name: String,
        description: String,
        required: bool,
    },
    StringEnumParam {
        name: String,
        description: String,
        required: bool,
        values: Vec<String>,
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
            ToolFunctionParameter::StringEnumParam { required, .. } => required,
            ToolFunctionParameter::ArrayParam { required, .. } => required,
            ToolFunctionParameter::IntegerParam { required, .. } => required,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            ToolFunctionParameter::StringParam { name, .. } => name,
            ToolFunctionParameter::StringEnumParam { name, .. } => name,
            ToolFunctionParameter::ArrayParam { name, .. } => name,
            ToolFunctionParameter::IntegerParam { name, .. } => name,
        }
    }
}

pub enum ArrayParamItemType {
    String,
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
                            required: _,
                        } => {
                            properties[name] = serde_json::json!({
                                "type": "string",
                                "description": description,
                            });
                        }
                        ToolFunctionParameter::StringEnumParam {
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
                        ToolFunctionParameter::ArrayParam {
                            name,
                            description,
                            item_type,
                            required: _,
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
