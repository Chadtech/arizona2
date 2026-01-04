use crate::capability::memory::{MemoryCapability, MemorySearchResult};
use crate::domain::memory_uuid::MemoryUuid;
use crate::domain::person_name::PersonName;
use crate::worker::Worker;
use crate::{admin_ui::s, capability, capability::memory::NewMemory};
use iced::{widget as w, Element, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    name_field: String,
    memory_field: String,
    status: Status,
    // Memory query fields
    query_person_recalling_field: String,
    query_people_field: String,
    query_scene_name_field: String,
    query_scene_description_field: String,
    query_recent_events_field: Vec<String>,
    query_state_of_mind_field: String,
    query_situation_field: String,
    query_status: QueryStatus,
}

enum Status {
    Ready,
    CreatingMemory,
    Done,
    FailedCreatingMemory(String),
}

#[derive(Clone, Debug)]
pub struct MemoryQueryResult {
    pub prompt: String,
    pub memories: Vec<MemorySearchResult>,
}

enum QueryStatus {
    Ready,
    GeneratingPrompt,
    Done(MemoryQueryResult),
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum Msg {
    NameFieldChanged(String),
    MemoryFieldChanged(String),
    ClickedCreateMemory,
    CreatedMemory(Result<MemoryUuid, String>),
    // Memory query messages
    QueryPersonRecallingChanged(String),
    QueryPeopleChanged(String),
    QuerySceneNameChanged(String),
    QuerySceneDescriptionChanged(String),
    QueryRecentEventChanged(usize, String),
    QueryStateOfMindChanged(String),
    QuerySituationChanged(String),
    ClickedAddRecentEvent,
    ClickedRemoveRecentEvent(usize),
    ClickedGeneratePrompt,
    GeneratedPromptAndSearched(Result<MemoryQueryResult, String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Storage {
    #[serde(default)]
    name_field: String,
    #[serde(default)]
    memory_field: String,
    #[serde(default)]
    query_person_recalling_field: String,
    #[serde(default)]
    query_people_field: String,
    #[serde(default)]
    query_scene_name_field: String,
    #[serde(default)]
    query_scene_description_field: String,
    #[serde(default)]
    query_recent_events_field: Vec<String>,
    #[serde(default)]
    query_state_of_mind_field: String,
    #[serde(default)]
    query_situation_field: String,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            name_field: String::new(),
            memory_field: String::new(),
            query_person_recalling_field: String::new(),
            query_people_field: String::new(),
            query_scene_name_field: String::new(),
            query_scene_description_field: String::new(),
            query_recent_events_field: Vec::new(),
            query_state_of_mind_field: String::new(),
            query_situation_field: String::new(),
        }
    }
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            name_field: storage.name_field.clone(),
            memory_field: storage.memory_field.clone(),
            status: Status::Ready,
            query_person_recalling_field: storage.query_person_recalling_field.clone(),
            query_people_field: storage.query_people_field.clone(),
            query_scene_name_field: storage.query_scene_name_field.clone(),
            query_scene_description_field: storage.query_scene_description_field.clone(),
            query_recent_events_field: storage.query_recent_events_field.clone(),
            query_state_of_mind_field: storage.query_state_of_mind_field.clone(),
            query_situation_field: storage.query_situation_field.clone(),
            query_status: QueryStatus::Ready,
        }
    }

    pub fn view(&self) -> Element<'_, Msg> {
        let mut col = w::column![
            w::text("Person Name"),
            w::text_input("", &self.name_field).on_input(Msg::NameFieldChanged),
            w::text("Memory"),
            w::text_input("", &self.memory_field).on_input(Msg::MemoryFieldChanged),
            w::button("Create Memory").on_press(Msg::ClickedCreateMemory),
            status_view(&self.status),
            w::horizontal_rule(1),
            w::text("Memory Query Prompt Generator").size(20),
            w::text("Person Recalling"),
            w::text_input("", &self.query_person_recalling_field)
                .on_input(Msg::QueryPersonRecallingChanged),
            w::text("People (comma-separated)"),
            w::text_input("", &self.query_people_field).on_input(Msg::QueryPeopleChanged),
            w::text("Scene Name"),
            w::text_input("", &self.query_scene_name_field).on_input(Msg::QuerySceneNameChanged),
            w::text("Scene Description"),
            w::text_input("", &self.query_scene_description_field)
                .on_input(Msg::QuerySceneDescriptionChanged),
            w::text("State of Mind"),
            w::text_input("", &self.query_state_of_mind_field)
                .on_input(Msg::QueryStateOfMindChanged),
            w::text("Situation"),
            w::text_input("", &self.query_situation_field).on_input(Msg::QuerySituationChanged),
            w::text("Recent Events"),
        ]
        .spacing(s::S4);

        // Add recent event fields
        for (i, event) in self.query_recent_events_field.iter().enumerate() {
            col = col.push(
                w::row![
                    w::text_input("", event).on_input(move |s| Msg::QueryRecentEventChanged(i, s)),
                    w::button("Remove").on_press(Msg::ClickedRemoveRecentEvent(i))
                ]
                .spacing(s::S4),
            );
        }

        col = col.push(w::button("Add Recent Event").on_press(Msg::ClickedAddRecentEvent));
        col = col.push(w::button("Generate Prompt").on_press(Msg::ClickedGeneratePrompt));
        col = col.push(query_status_view(&self.query_status));

        col.into()
    }

    pub fn to_storage(&self) -> Storage {
        Storage {
            name_field: self.name_field.clone(),
            memory_field: self.memory_field.clone(),
            query_person_recalling_field: self.query_person_recalling_field.clone(),
            query_people_field: self.query_people_field.clone(),
            query_scene_name_field: self.query_scene_name_field.clone(),
            query_scene_description_field: self.query_scene_description_field.clone(),
            query_recent_events_field: self.query_recent_events_field.clone(),
            query_state_of_mind_field: self.query_state_of_mind_field.clone(),
            query_situation_field: self.query_situation_field.clone(),
        }
    }

    pub fn update(&mut self, worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::NameFieldChanged(field) => {
                self.name_field = field;
                Task::none()
            }
            Msg::MemoryFieldChanged(field) => {
                self.memory_field = field;
                Task::none()
            }
            Msg::ClickedCreateMemory => {
                self.status = Status::CreatingMemory;

                let new_memory = NewMemory {
                    memory_uuid: MemoryUuid::new(),
                    content: self.memory_field.clone(),
                    person_name: PersonName::from_string(self.name_field.clone()),
                };

                Task::perform(
                    async move { create_new_memory(&worker, new_memory).await },
                    Msg::CreatedMemory,
                )
            }
            Msg::CreatedMemory(result) => {
                self.status = match result {
                    Ok(_) => Status::Done,
                    Err(err) => Status::FailedCreatingMemory(err),
                };

                Task::none()
            }
            Msg::QueryPersonRecallingChanged(value) => {
                self.query_person_recalling_field = value;
                Task::none()
            }
            Msg::QueryPeopleChanged(value) => {
                self.query_people_field = value;
                Task::none()
            }
            Msg::QuerySceneNameChanged(value) => {
                self.query_scene_name_field = value;
                Task::none()
            }
            Msg::QuerySceneDescriptionChanged(value) => {
                self.query_scene_description_field = value;
                Task::none()
            }
            Msg::QueryRecentEventChanged(index, value) => {
                if index < self.query_recent_events_field.len() {
                    self.query_recent_events_field[index] = value;
                }
                Task::none()
            }
            Msg::QueryStateOfMindChanged(value) => {
                self.query_state_of_mind_field = value;
                Task::none()
            }
            Msg::QuerySituationChanged(value) => {
                self.query_situation_field = value;
                Task::none()
            }
            Msg::ClickedAddRecentEvent => {
                self.query_recent_events_field.push(String::new());
                Task::none()
            }
            Msg::ClickedRemoveRecentEvent(index) => {
                if index < self.query_recent_events_field.len() {
                    self.query_recent_events_field.remove(index);
                }
                Task::none()
            }
            Msg::ClickedGeneratePrompt => {
                self.query_status = QueryStatus::GeneratingPrompt;

                let person_recalling =
                    PersonName::from_string(self.query_person_recalling_field.clone());
                let people: Vec<String> = self
                    .query_people_field
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let scene_name = self.query_scene_name_field.clone();
                let scene_description = self.query_scene_description_field.clone();
                let recent_events = self.query_recent_events_field.clone();
                let state_of_mind = self.query_state_of_mind_field.clone();
                let situation = self.query_situation_field.clone();

                Task::perform(
                    async move {
                        generate_prompt_and_search_memories(
                            &worker,
                            person_recalling,
                            people,
                            scene_name,
                            scene_description,
                            recent_events,
                            state_of_mind,
                            situation,
                        )
                        .await
                    },
                    Msg::GeneratedPromptAndSearched,
                )
            }
            Msg::GeneratedPromptAndSearched(result) => {
                self.query_status = match result {
                    Ok(query_result) => QueryStatus::Done(query_result),
                    Err(err) => QueryStatus::Failed(err),
                };

                Task::none()
            }
        }
    }
}

fn status_view(status: &Status) -> Element<'_, Msg> {
    match status {
        Status::Ready => w::text("Ready").into(),
        Status::CreatingMemory => w::text("Creating Memory...").into(),
        Status::Done => w::text("Memory created successfully!").into(),
        Status::FailedCreatingMemory(err) => w::text(format!("Error: {}", err)).into(),
    }
}

async fn create_new_memory(worker: &Worker, new_memory: NewMemory) -> Result<MemoryUuid, String> {
    worker.create_memory(new_memory).await
}

fn query_status_view(status: &QueryStatus) -> Element<'_, Msg> {
    match status {
        QueryStatus::Ready => w::text("Ready to generate prompt").into(),
        QueryStatus::GeneratingPrompt => w::text("Generating prompt...").into(),
        QueryStatus::Done(result) => {
            let mut col = w::column![
                w::text("Generated Prompt:").size(16),
                w::text(&result.prompt),
                w::horizontal_rule(1),
                w::text(format!("Found {} memories:", result.memories.len())).size(16),
            ]
            .spacing(s::S4);

            for (i, memory) in result.memories.iter().enumerate() {
                col = col.push(
                    w::column![
                        w::text(format!(
                            "Memory {} (distance: {:.3})",
                            i + 1,
                            memory.distance
                        )),
                        w::text(&memory.content),
                        w::horizontal_rule(1),
                    ]
                    .spacing(s::S2),
                );
            }

            col.into()
        }
        QueryStatus::Failed(err) => w::text(format!("Error: {}", err)).into(),
    }
}

async fn generate_prompt_and_search_memories(
    worker: &Worker,
    person_recalling: PersonName,
    people: Vec<String>,
    scene_name: String,
    scene_description: String,
    recent_events: Vec<String>,
    state_of_mind: String,
    situation: String,
) -> Result<MemoryQueryResult, String> {
    let message_type_args = capability::memory::MessageTypeArgs::Scene {
        scene_name: scene_name.clone(),
        scene_description: scene_description.clone(),
        people,
    };
    // Generate the prompt
    let prompt_result = worker
        .create_memory_query_prompt(
            person_recalling,
            message_type_args,
            recent_events,
            &state_of_mind,
            &situation,
        )
        .await?;

    // Search for memories using the generated prompt
    let memories = worker
        .search_memories(prompt_result.prompt.clone(), 10)
        .await?;

    Ok(MemoryQueryResult {
        prompt: prompt_result.prompt,
        memories,
    })
}
