use super::call;
use super::style as s;
use crate::capability::person::PersonCapability;
use crate::capability::reaction::{ReactionCapability, ReactionPromptPreview};
use crate::domain::memory::Memory;
use crate::domain::person_name::PersonName;
use crate::nice_display::NiceDisplay;
use crate::open_ai::completion::CompletionError;
use crate::person_actions::PersonReaction;
use crate::worker::Worker;
use iced::{widget as w, Element, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    person_name_field: String,
    identity_field: String,
    memory_fields: Vec<w::text_editor::Content>,
    situation_field: String,
    state_of_mind_field: String,
    reaction_status: ReactionStatus,
}

enum ReactionStatus {
    Ready,
    Response(Vec<PersonReaction>),
    PromptPreview(ReactionPromptPreview),
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Storage {
    #[serde(default)]
    pub person_name_field: String,
    #[serde(default)]
    pub identity_field: String,
    #[serde(default)]
    pub memories: Vec<String>,
    #[serde(default)]
    pub situation_field: String,
    #[serde(default)]
    pub state_of_mind_field: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    ClickedAddMemory,
    MemoryUpdated {
        index: usize,
        action: w::text_editor::Action,
    },
    PersonNameFieldChanged(String),
    IdentityFieldChanged(String),
    ClickedPreviewPrompts,
    ClickedSubmitReaction,
    SituationFieldChanged(String),
    StateOfMindFieldChanged(String),
    ReactionSubmissionResult(Result<Vec<PersonReaction>, CompletionError>),
    PromptPreviewResult(Result<ReactionPromptPreview, String>),
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Model {
            person_name_field: storage.person_name_field.clone(),
            identity_field: storage.identity_field.clone(),
            memory_fields: storage
                .memories
                .iter()
                .map(|content_str| w::text_editor::Content::with_text(content_str))
                .collect(),
            situation_field: storage.situation_field.clone(),
            state_of_mind_field: storage.state_of_mind_field.clone(),
            reaction_status: ReactionStatus::Ready,
        }
    }

    pub fn to_storage(&self) -> Storage {
        Storage {
            person_name_field: self.person_name_field.clone(),
            identity_field: self.identity_field.clone(),
            memories: self
                .memory_fields
                .iter()
                .map(|editor_content| editor_content.text())
                .collect(),
            situation_field: self.situation_field.clone(),
            state_of_mind_field: self.state_of_mind_field.clone(),
        }
    }

    pub fn update(&mut self, worker: Arc<Worker>, message: Msg) -> Task<Msg> {
        match message {
            Msg::ClickedAddMemory => {
                self.memory_fields.push(w::text_editor::Content::new());
                Task::none()
            }
            Msg::MemoryUpdated { index, action } => {
                if let Some(memory) = self.memory_fields.get_mut(index) {
                    memory.perform(action);
                }
                Task::none()
            }
            Msg::PersonNameFieldChanged(new_field) => {
                self.person_name_field = new_field;
                Task::none()
            }
            Msg::IdentityFieldChanged(new_field) => {
                self.identity_field = new_field;
                Task::none()
            }
            Msg::ClickedPreviewPrompts => {
                let person_name = self.person_name_field.clone();
                let memories = self
                    .memory_fields
                    .iter()
                    .map(|editor_content| Memory {
                        content: editor_content.text(),
                    })
                    .collect::<Vec<Memory>>();
                let situation = self.situation_field.clone();

                Task::perform(
                    async move {
                        preview_reaction_prompts(&worker, person_name, memories, situation).await
                    },
                    Msg::PromptPreviewResult,
                )
            }
            Msg::ClickedSubmitReaction => {
                let open_ai_key = worker.open_ai_key.clone();
                let memories: Vec<String> = self
                    .memory_fields
                    .iter()
                    .map(|editor_content| editor_content.text())
                    .collect();
                let person_identity = self.identity_field.clone();
                let situation = self.situation_field.clone();
                let state_of_mind = self.state_of_mind_field.clone();

                Task::perform(
                    call::submit_reaction(
                        open_ai_key,
                        memories,
                        person_identity,
                        situation,
                        state_of_mind,
                    ),
                    Msg::ReactionSubmissionResult,
                )
            }
            Msg::SituationFieldChanged(new_field) => {
                self.situation_field = new_field;
                Task::none()
            }
            Msg::StateOfMindFieldChanged(new_field) => {
                self.state_of_mind_field = new_field;
                Task::none()
            }
            Msg::ReactionSubmissionResult(result) => {
                self.reaction_status = match result {
                    Ok(response) => ReactionStatus::Response(response),
                    Err(err) => ReactionStatus::Error(err.to_nice_error().to_string()),
                };
                Task::none()
            }
            Msg::PromptPreviewResult(result) => {
                self.reaction_status = match result {
                    Ok(preview) => ReactionStatus::PromptPreview(preview),
                    Err(err) => ReactionStatus::Error(err),
                };
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Msg> {
        let mut memories_children: Vec<Element<_>> = vec![];

        for (i, memory) in self.memory_fields.iter().enumerate() {
            let memories_editor = w::text_editor(memory).on_action(move |act| Msg::MemoryUpdated {
                index: i,
                action: act,
            });

            memories_children.push(memories_editor.into());
        }

        let reaction_response_view: Element<Msg> = match &self.reaction_status {
            ReactionStatus::Ready => w::Column::new().into(),
            ReactionStatus::Response(response) => w::Column::with_children(
                response
                    .iter()
                    .map(|reaction| w::text(format!("Reaction: {:#?}", reaction)).into())
                    .collect::<Vec<_>>(),
            )
            .into(),
            ReactionStatus::PromptPreview(preview) => w::column![
                w::text("Thinking System Prompt"),
                w::text(&preview.thinking_system_prompt),
                w::horizontal_rule(1),
                w::text("Thinking User Prompt"),
                w::text(&preview.thinking_user_prompt),
                w::horizontal_rule(1),
                w::text("Action System Prompt"),
                w::text(&preview.action_system_prompt),
                w::horizontal_rule(1),
                w::text("Action User Prompt"),
                w::text(&preview.action_user_prompt),
            ]
            .spacing(s::S2)
            .into(),
            ReactionStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
        };

        w::column![
            w::text("Person Name"),
            w::text_input("Person Name", &self.person_name_field)
                .on_input(Msg::PersonNameFieldChanged),
            w::text("Identity"),
            w::text_input("Identity", &self.identity_field).on_input(Msg::IdentityFieldChanged),
            w::text("Memories"),
            w::Column::with_children(memories_children).spacing(s::S4),
            w::button("Add Memory").on_press(Msg::ClickedAddMemory),
            w::text("Situation"),
            w::text_input("Situation", &self.situation_field).on_input(Msg::SituationFieldChanged),
            w::text("State of Mind"),
            w::text_input("State of Mind", &self.state_of_mind_field)
                .on_input(Msg::StateOfMindFieldChanged),
            w::button("Preview Prompts (No LLM)").on_press(Msg::ClickedPreviewPrompts),
            w::button("Submit Reaction").on_press(Msg::ClickedSubmitReaction),
            reaction_response_view,
        ]
        .spacing(s::S4)
        .into()
    }
}

async fn preview_reaction_prompts(
    worker: &Worker,
    person_name: String,
    memories: Vec<Memory>,
    situation: String,
) -> Result<ReactionPromptPreview, String> {
    let person_uuid = worker
        .get_person_uuid_by_name(PersonName::from_string(person_name))
        .await?;

    worker
        .preview_reaction_prompts(memories, person_uuid, situation)
        .await
}
