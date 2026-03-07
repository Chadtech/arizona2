use crate::capability::motivation::MotivationCapability;
use crate::capability::person::PersonCapability;
use crate::capability::reflection::{ReflectionCapability, ReflectionChange};
use crate::domain::logger::Level;
use crate::domain::memory::Memory;
use crate::domain::motivation_uuid::MotivationUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::nice_display::NiceDisplay;
use crate::open_ai;
use crate::open_ai::completion::Completion;
use crate::open_ai::role::Role;
use crate::open_ai::tool::{ToolFunction, ToolFunctionParameter};
use crate::open_ai::tool_call::ToolCall;
use crate::worker::Worker;
use std::collections::HashMap;

enum ChangeOption {
    StateOfMind,
    MemorySummary,
    NewMotivation,
    DeleteMotivation,
}

impl ChangeOption {
    fn all_options() -> Vec<ChangeOption> {
        vec![
            ChangeOption::StateOfMind,
            ChangeOption::MemorySummary,
            ChangeOption::NewMotivation,
            ChangeOption::DeleteMotivation,
        ]
    }

    fn to_tool(&self) -> ToolFunction {
        match self {
            ChangeOption::StateOfMind => ToolFunction::new(
                "update_state_of_mind".to_string(),
                "Update the person's state of mind after reflection.".to_string(),
                vec![ToolFunctionParameter::String {
                    name: "content".to_string(),
                    description: "A neutral, direct statement of internal state only. Use third person. Do not use first person, comparative/relative wording, or references to specific events/people/actions."
                        .to_string(),
                    required: true,
                }],
            ),
            ChangeOption::MemorySummary => ToolFunction::new(
                "summarize_memories".to_string(),
                "Summarize relevant memories into a concise reflection.".to_string(),
                vec![ToolFunctionParameter::String {
                    name: "summary".to_string(),
                    description: "A concise, first-person summary that combines related memories."
                        .to_string(),
                    required: true,
                }],
            ),
            ChangeOption::NewMotivation => ToolFunction::new(
                "add_motivation".to_string(),
                "Add a new motivation for the person.".to_string(),
                vec![
                    ToolFunctionParameter::String {
                        name: "content".to_string(),
                        description: "The motivation content.".to_string(),
                        required: true,
                    },
                    ToolFunctionParameter::Integer {
                        name: "priority".to_string(),
                        description: "Priority for the motivation (higher = more important)."
                            .to_string(),
                        required: true,
                    },
                ],
            ),
            ChangeOption::DeleteMotivation => ToolFunction::new(
                "remove_motivation".to_string(),
                "Remove a motivation using the index from the enumerated motivations list."
                    .to_string(),
                vec![ToolFunctionParameter::Integer {
                    name: "index".to_string(),
                    description: "The index from the motivations list to remove.".to_string(),
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

        let motivations = self
            .get_motivations_for_person(person_uuid.clone())
            .await
            .map_err(|err| format!("Failed to get motivations: {}", err))?;

        let motivation_index_map: HashMap<usize, MotivationUuid> = motivations
            .iter()
            .enumerate()
            .map(|(index, motivation)| (index + 1, motivation.uuid.clone()))
            .collect();

        let motivations_list = if motivations.is_empty() {
            "None.".to_string()
        } else {
            motivations
                .iter()
                .enumerate()
                .map(|(index, motivation)| {
                    format!(
                        "{}. (priority {}) {}",
                        index + 1,
                        motivation.priority,
                        motivation.content
                    )
                })
                .collect::<Vec<String>>()
                .join("\n")
        };

        let user_prompt = format!(
            "Person: {}\n\nPerson identity:\n{}\n\nState of mind:\n{}\n\nMemories:\n{}\n\nMotivations:\n{}\n\nSituation:\n{}",
            person_name.as_str(),
            person_identity,
            state_of_mind,
            Memory::many_to_list_text(&memories),
            motivations_list,
            situation
        );

        self.logger.log(
            Level::Info,
            format!("Reflection prompt:\n{}", user_prompt).as_str(),
        );

        let mut completion = Completion::new(open_ai::model::Model::DEFAULT);
        completion.add_message(
            Role::System,
            "You are a reflection assistant making an objective, third-person assessment of how this person's mind would realistically change after reflecting on the situation. Predict natural, human shifts rather than idealized outcomes. For state of mind updates, output a neutral, direct statement of internal disposition only. Rules for state of mind: third person only; no first-person words (I/me/my/we/us/our); no comparative or relative phrasing (e.g., calmer, steadier, more, less, better, worse, than); no references to specific events, invitations, conversations, or what anyone said/did. Keep it abstract and trait-level (e.g., \"steady and guarded\", \"restless and distracted\", \"cautious but curious\"). Memory summaries may include concrete details. If removing a motivation, use the index from the enumerated motivations list. Decide whether to update their state of mind, summarize memories, or adjust motivations. Use a tool call only when there is a meaningful change. If nothing should change, do not call any tools. Respond with tool calls only. Consider whether any motivations should be added or removed based on the situation, especially when feedback suggests a long-term mismatch or feasibility constraint.",
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
                    let change = reflection_change_from_tool_call(call, &motivation_index_map)?;
                    changes.push(change);
                }

                let change_summary = changes
                    .iter()
                    .map(describe_reflection_change)
                    .collect::<Vec<String>>()
                    .join("\n");
                self.logger.log(
                    Level::Info,
                    format!("Reflection changes:\n{}", change_summary).as_str(),
                );

                Ok(changes)
            }
        }
    }
}

fn describe_reflection_change(change: &ReflectionChange) -> String {
    match change {
        ReflectionChange::StateOfMind { content } => {
            format!("StateOfMind: {}", content)
        }
        ReflectionChange::MemorySummary { summary } => {
            format!("MemorySummary: {}", summary)
        }
        ReflectionChange::NewMotivation { content, priority } => {
            format!("NewMotivation (priority {}): {}", priority, content)
        }
        ReflectionChange::DeleteMotivation { motivation_uuid } => {
            format!("DeleteMotivation: {}", motivation_uuid.to_uuid())
        }
    }
}

fn reflection_change_from_tool_call(
    call: ToolCall,
    motivation_index_map: &HashMap<usize, MotivationUuid>,
) -> Result<ReflectionChange, String> {
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
        "add_motivation" => {
            let content = call
                .arguments
                .iter()
                .find(|(name, _)| name == "content")
                .and_then(|(_, value)| value.as_str())
                .ok_or_else(|| {
                    "Missing 'content' argument in 'add_motivation' tool call".to_string()
                })?;
            let priority = call
                .arguments
                .iter()
                .find(|(name, _)| name == "priority")
                .and_then(|(_, value)| value.as_i64())
                .ok_or_else(|| {
                    "Missing 'priority' argument in 'add_motivation' tool call".to_string()
                })?;
            Ok(ReflectionChange::NewMotivation {
                content: content.to_string(),
                priority,
            })
        }
        "remove_motivation" => {
            let index = call
                .arguments
                .iter()
                .find(|(name, _)| name == "index")
                .and_then(|(_, value)| value.as_u64())
                .ok_or_else(|| {
                    "Missing 'index' argument in 'remove_motivation' tool call".to_string()
                })?;
            let index = index as usize;
            let motivation_uuid = motivation_index_map
                .get(&index)
                .ok_or_else(|| format!("Unknown motivation index {}", index))?;
            Ok(ReflectionChange::DeleteMotivation {
                motivation_uuid: motivation_uuid.clone(),
            })
        }
        other => Err(format!("Unrecognized reflection tool call: {}", other)),
    }
}
