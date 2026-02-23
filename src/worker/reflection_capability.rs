use crate::capability::person::PersonCapability;
use crate::capability::reflection::{ReflectionCapability, ReflectionChange};
use crate::domain::memory::Memory;
use crate::domain::person_uuid::PersonUuid;
use crate::nice_display::NiceDisplay;
use crate::open_ai;
use crate::open_ai::completion::Completion;
use crate::open_ai::role::Role;
use crate::open_ai::tool::{ToolFunction, ToolFunctionParameter};
use crate::open_ai::tool_call::ToolCall;
use crate::worker::Worker;

enum ChangeOption {
    StateOfMind,
    MemorySummary,
}

impl ChangeOption {
    fn all_options() -> Vec<ChangeOption> {
        vec![ChangeOption::StateOfMind, ChangeOption::MemorySummary]
    }

    fn to_tool(&self) -> ToolFunction {
        match self {
            ChangeOption::StateOfMind => ToolFunction::new(
                "update_state_of_mind".to_string(),
                "Update the person's state of mind after reflection.".to_string(),
                vec![ToolFunctionParameter::StringParam {
                    name: "content".to_string(),
                    description: "The new state of mind, written in the person's voice."
                        .to_string(),
                    required: true,
                }],
            ),
            ChangeOption::MemorySummary => ToolFunction::new(
                "summarize_memories".to_string(),
                "Summarize relevant memories into a concise reflection.".to_string(),
                vec![ToolFunctionParameter::StringParam {
                    name: "summary".to_string(),
                    description: "A concise, first-person summary that combines related memories."
                        .to_string(),
                    required: true,
                }],
            ),
        }
    }
}

impl ReflectionCapability for Worker {
    async fn get_reflection_changes(
        &self,
        memories: Vec<Memory>,
        person_uuid: PersonUuid,
        person_identity: String,
        state_of_mind: String,
        situation: String,
    ) -> Result<Vec<ReflectionChange>, String> {
        let person_name = self
            .get_persons_name(person_uuid.clone())
            .await
            .map_err(|err| format!("Failed to get person name: {}", err))?;

        let memories_list = if memories.is_empty() {
            "None.".to_string()
        } else {
            memories
                .iter()
                .map(|memory| format!("- {}", memory.content))
                .collect::<Vec<String>>()
                .join("\n")
        };

        let user_prompt = format!(
            "Person: {}\n\nPerson identity:\n{}\n\nState of mind:\n{}\n\nMemories:\n{}\n\nSituation:\n{}",
            person_name.as_str(),
            person_identity,
            state_of_mind,
            memories_list,
            situation
        );

        let mut completion = Completion::new(open_ai::model::Model::DEFAULT);
        completion.add_message(
            Role::System,
            "You are a reflection assistant making an objective, third-person assessment of how this person's mind would realistically change after reflecting on the situation. Predict natural, human shifts rather than idealized outcomes. Decide whether to update their state of mind or summarize memories. Use a tool call only when there is a meaningful change. If nothing should change, do not call any tools. Respond with tool calls only.",
        );
        completion.add_message(Role::User, user_prompt.as_str());

        for option in ChangeOption::all_options() {
            completion.add_tool_call(option.to_tool().into());
        }

        let response = completion
            .send_request(&self.open_ai_key, self.reqwest_client.clone())
            .await
            .map_err(|err| err.message())?;

        let tool_calls = response.maybe_tool_calls().map_err(|err| err.message())?;

        let mut changes = Vec::new();
        match tool_calls {
            None => Ok(changes),
            Some(tool_calls) => {
                for call in tool_calls {
                    let change = reflection_change_from_tool_call(call)?;
                    changes.push(change);
                }
                Ok(changes)
            }
        }
    }
}

fn reflection_change_from_tool_call(call: ToolCall) -> Result<ReflectionChange, String> {
    match call.name.as_str() {
        "update_state_of_mind" => {
            let content = call
                .arguments
                .iter()
                .find(|(name, _)| name == "content")
                .and_then(|(_, value)| value.as_str())
                .ok_or_else(|| {
                    "Missing 'content' argument in 'update_state_of_mind' tool call".to_string()
                })?;
            Ok(ReflectionChange::StateOfMind {
                content: content.to_string(),
            })
        }
        "summarize_memories" => {
            let summary = call
                .arguments
                .iter()
                .find(|(name, _)| name == "summary")
                .and_then(|(_, value)| value.as_str())
                .ok_or_else(|| {
                    "Missing 'summary' argument in 'summarize_memories' tool call".to_string()
                })?;
            Ok(ReflectionChange::MemorySummary {
                summary: summary.to_string(),
            })
        }
        other => Err(format!("Unrecognized reflection tool call: {}", other)),
    }
}
