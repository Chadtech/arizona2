mod call;
mod job_page;
mod memory_page;
mod new_identity_page;
mod new_person_page;
mod scene_page;
mod state_of_mind_page;
mod style;

use self::style as s;
use crate::nice_display::NiceDisplay;
use crate::open_ai::completion::CompletionError;
use crate::person_actions::PersonAction;
use crate::worker;
use crate::worker::Worker;
use iced;
use iced::{widget as w, Color, Element, Task, Theme};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

const STORAGE_FILE_PATH: &str = "storage.json";

struct Model {
    prompt_field: String,
    identity_field: String,
    prompt_status: PromptStatus,
    memory_fields: Vec<w::text_editor::Content>,
    situation_field: String,
    state_of_mind_field: String,
    reaction_status: ReactionStatus,
    new_identity_page: new_identity_page::Model,
    new_person_page: new_person_page::Model,
    memory_page: memory_page::Model,
    state_of_mind_page: state_of_mind_page::Model,
    scene_page: scene_page::Model,
    job_page: job_page::Model,
    tab: Tab,
    worker: Arc<Worker>,
    error: Option<Error>,
}

impl Model {
    pub fn to_storage(&self) -> Storage {
        Storage {
            prompt: self.prompt_field.clone(),
            memories: self
                .memory_fields
                .iter()
                .map(|editor_content| editor_content.text())
                .collect(),
            identity_field: self.identity_field.clone(),
            situation_field: self.situation_field.clone(),
            state_of_mind_field: self.state_of_mind_field.clone(),
            new_identity: self.new_identity_page.to_storage(),
            new_person: self.new_person_page.to_storage(),
            memory: self.memory_page.to_storage(),
            state_of_mind: self.state_of_mind_page.to_storage(),
            scene: self.scene_page.to_storage(),
            job: self.job_page.to_storage(),
            tab: self.tab.clone(),
        }
    }
}

enum PromptStatus {
    Ready,
    Response(String),
    Error(CompletionError),
}

enum ReactionStatus {
    Ready,
    Response(Vec<PersonAction>),
    Error(CompletionError),
}

#[derive(Serialize, Deserialize, Debug)]
struct Storage {
    prompt: String,
    #[serde(default)]
    memories: Vec<String>,
    #[serde(default)]
    identity_field: String,
    #[serde(default)]
    tab: Tab,
    #[serde(default)]
    situation_field: String,
    #[serde(default)]
    state_of_mind_field: String,
    #[serde(default)]
    new_identity: new_identity_page::Storage,
    #[serde(default)]
    new_person: new_person_page::Storage,
    #[serde(default)]
    memory: memory_page::Storage,
    #[serde(default)]
    state_of_mind: state_of_mind_page::Storage,
    #[serde(default)]
    scene: scene_page::Storage,
    #[serde(default)]
    job: job_page::Storage,
}

impl Storage {
    pub fn save_to_file_system(&self) -> Result<(), Error> {
        // Serialize the Storage struct to JSON
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| Error::StorageSerializationError(e.to_string()))?;

        // Create a file to save the JSON
        let path = Path::new(STORAGE_FILE_PATH);
        let mut file = File::create(path).map_err(Error::StorageFileCreationError)?;

        // Write the JSON to the file
        file.write_all(json.as_bytes())
            .map_err(Error::StorageFileWriteError)?;

        Ok(())
    }

    pub fn read_from_file_system() -> Result<Self, Error> {
        // Check if the file exists
        let path = Path::new(STORAGE_FILE_PATH);
        if !path.exists() {
            return Ok(Self::default());
        }

        // Open the file
        let file = File::open(path).map_err(Error::StorageFileReadError)?;

        // Deserialize the JSON into a Storage struct
        let storage: Storage = serde_json::from_reader(file)
            .map_err(|e| Error::StorageDeserializationError(e.to_string()))?;

        Ok(storage)
    }

    pub fn default() -> Self {
        Storage {
            prompt: String::new(),
            memories: Vec::new(),
            identity_field: String::new(),
            tab: Tab::default(),
            situation_field: String::new(),
            state_of_mind_field: String::new(),
            new_identity: new_identity_page::Storage::default(),
            new_person: new_person_page::Storage::default(),
            memory: memory_page::Storage::default(),
            state_of_mind: state_of_mind_page::Storage::default(),
            scene: scene_page::Storage::default(),
            job: job_page::Storage::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
enum Tab {
    Prompt,
    Reaction,
    Identity,
    Person,
    Memory,
    StateOfMind,
    Scene,
    Job,
}

impl Tab {
    pub fn to_label(&self) -> String {
        match self {
            Tab::Prompt => "Prompt".to_string(),
            Tab::Reaction => "Reaction".to_string(),
            Tab::Identity => "Identity".to_string(),
            Tab::Person => "Person".to_string(),
            Tab::Memory => "Memory".to_string(),
            Tab::StateOfMind => "State of Mind".to_string(),
            Tab::Scene => "Scene".to_string(),
            Tab::Job => "Job".to_string(),
        }
    }

    pub fn all() -> Vec<Tab> {
        vec![
            Tab::Prompt,
            Tab::Reaction,
            Tab::Identity,
            Tab::Person,
            Tab::Memory,
            Tab::StateOfMind,
            Tab::Scene,
            Tab::Job,
        ]
    }

    pub fn init_task(self, worker: &Arc<Worker>) -> Task<Msg> {
        if self == Tab::Job {
            Task::perform(job_page::get_jobs(worker.clone()), Msg::JobPageMsg)
        } else {
            Task::none()
        }
    }
}

impl Default for Tab {
    fn default() -> Self {
        Tab::Prompt
    }
}

#[derive(Debug)]
struct Flags {
    worker: Worker,
    storage: Storage,
}

impl Flags {
    async fn get() -> Result<Self, Error> {
        let worker = Worker::new().await.map_err(Error::WorkerInitError)?;

        let storage = Storage::read_from_file_system()?;

        Ok(Flags { worker, storage })
    }
}

#[derive(Debug, Clone)]
enum Msg {
    PromptFieldChanged(String),
    ClickedSubmitPrompt,
    ClickedAddMemory,
    SubmissionResult(Result<String, CompletionError>),
    MemoryUpdated {
        index: usize,
        action: w::text_editor::Action,
    },
    IdentityFieldChanged(String),
    TabSelected(Tab),
    ClickedSubmitReaction,
    SituationFieldChanged(String),
    StateOfMindFieldChanged(String),
    ReactionSubmissionResult(Result<Vec<PersonAction>, CompletionError>),
    NewIdentityPageMsg(new_identity_page::Msg),
    NewPersonPageMsg(new_person_page::Msg),
    MemoryPageMsg(memory_page::Msg),
    StateOfMindPageMsg(state_of_mind_page::Msg),
    SceneMsg(scene_page::Msg),
    JobPageMsg(job_page::Msg),
    WarmedUpDb,
}

#[derive(Debug)]
pub enum Error {
    IcedRunError(iced::Error),
    WorkerInitError(worker::InitError),
    StorageFileCreationError(std::io::Error),
    StorageFileWriteError(std::io::Error),
    StorageSerializationError(String),
    StorageFileReadError(std::io::Error),
    StorageDeserializationError(String),
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::IcedRunError(err) => format!("Iced run error: {}", err),
            Error::WorkerInitError(err) => err.message(),
            Error::StorageFileCreationError(err) => format!("Storage file creation error: {}", err),
            Error::StorageFileWriteError(err) => format!("Storage file write error: {}", err),
            Error::StorageSerializationError(msg) => {
                format!("Storage serialization error: {}", msg)
            }
            Error::StorageFileReadError(err) => format!("Storage file read error: {}", err),
            Error::StorageDeserializationError(msg) => {
                format!("Storage deserialization error: {}", msg)
            }
        }
    }
}

impl Model {
    fn new(flags: Flags) -> (Self, Task<Msg>) {
        let tab = flags.storage.tab;

        let model = Model {
            prompt_field: flags.storage.prompt,
            identity_field: flags.storage.identity_field,
            prompt_status: PromptStatus::Ready,
            memory_fields: flags
                .storage
                .memories
                .iter()
                .map(|content_str| w::text_editor::Content::with_text(content_str))
                .collect(),
            situation_field: flags.storage.situation_field,
            state_of_mind_field: flags.storage.state_of_mind_field,
            new_identity_page: new_identity_page::Model::new(&flags.storage.new_identity),
            new_person_page: new_person_page::Model::new(&flags.storage.new_person),
            memory_page: memory_page::Model::new(&flags.storage.memory),
            scene_page: scene_page::Model::new(&flags.storage.scene),
            job_page: job_page::Model::new(&flags.storage.job),
            reaction_status: ReactionStatus::Ready,
            tab,
            worker: Arc::new(flags.worker),
            error: None,
            state_of_mind_page: state_of_mind_page::Model::new(&flags.storage.state_of_mind),
        };

        let worker2 = model.worker.clone();

        let tab_task = tab.init_task(&model.worker);

        (
            model,
            Task::batch(vec![
                Task::perform(async move { worker2.warm_up_db_connection().await }, |_| {
                    Msg::WarmedUpDb
                }),
                tab_task,
            ]),
        )
    }

    fn title(&self) -> String {
        "Arizona 2 Admin".to_string()
    }

    fn update(&mut self, message: Msg) -> Task<Msg> {
        match message {
            Msg::PromptFieldChanged(field) => {
                self.prompt_field = field;

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                Task::none()
            }
            Msg::ClickedSubmitPrompt => {
                let open_ai_key = self.worker.open_ai_key.clone();
                let reqwest_client = self.worker.reqwest_client.clone();
                Task::perform(
                    call::submit_prompt(open_ai_key, reqwest_client, self.prompt_field.clone()),
                    Msg::SubmissionResult,
                )
            }
            Msg::SubmissionResult(result) => {
                self.prompt_status = match result {
                    Ok(response) => PromptStatus::Response(response.clone()),
                    Err(err) => PromptStatus::Error(err.clone()),
                };

                Task::none()
            }
            Msg::ClickedAddMemory => {
                self.memory_fields.push(w::text_editor::Content::new());

                // Save the updated memories to the file system
                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                Task::none()
            }
            Msg::MemoryUpdated { index, action } => {
                if let Some(memory) = self.memory_fields.get_mut(index) {
                    memory.perform(action);

                    // Save the updated memories to the file system
                    if let Err(err) = self.to_storage().save_to_file_system() {
                        self.error = Some(err);
                    }
                }

                Task::none()
            }
            Msg::IdentityFieldChanged(new_field) => {
                self.identity_field = new_field;

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                Task::none()
            }
            Msg::TabSelected(tab) => {
                self.tab = tab.clone();

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                self.tab.init_task(&self.worker)
            }
            Msg::ClickedSubmitReaction => {
                let open_ai_key = self.worker.open_ai_key.clone();
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

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                Task::none()
            }
            Msg::StateOfMindFieldChanged(new_field) => {
                self.state_of_mind_field = new_field;

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                Task::none()
            }
            Msg::ReactionSubmissionResult(result) => {
                self.reaction_status = match result {
                    Ok(response) => ReactionStatus::Response(response.clone()),
                    Err(err) => ReactionStatus::Error(err.clone()),
                };

                Task::none()
            }
            Msg::NewIdentityPageMsg(sub_msg) => {
                let task = self.new_identity_page.update(self.worker.clone(), sub_msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::NewIdentityPageMsg)
            }
            Msg::NewPersonPageMsg(sub_msg) => {
                let task = self.new_person_page.update(self.worker.clone(), sub_msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::NewPersonPageMsg)
            }
            Msg::MemoryPageMsg(sub_msg) => {
                let task = self.memory_page.update(self.worker.clone(), sub_msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::MemoryPageMsg)
            }
            Msg::WarmedUpDb => Task::none(),
            Msg::StateOfMindPageMsg(msg) => {
                let task = self.state_of_mind_page.update(self.worker.clone(), msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::StateOfMindPageMsg)
            }
            Msg::SceneMsg(sub_msg) => {
                let task = self.scene_page.update(self.worker.clone(), sub_msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::SceneMsg)
            }
            Msg::JobPageMsg(sub_msg) => {
                let task = self.job_page.update(self.worker.clone(), sub_msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::JobPageMsg)
            }
        }
    }

    fn view(&self) -> Element<Msg> {
        let mut memories_children: Vec<Element<_>> = vec![];

        for (i, memory) in self.memory_fields.iter().enumerate() {
            let memories_editor = w::text_editor(memory).on_action(move |act| Msg::MemoryUpdated {
                index: i,
                action: act,
            });

            memories_children.push(memories_editor.into());
        }

        let tabs = Tab::all()
            .iter()
            .map(|tab: &Tab| {
                w::radio(
                    tab.to_label(),
                    tab.clone(),
                    Some(self.tab.clone()),
                    Msg::TabSelected,
                )
                .into()
            })
            .collect::<Vec<Element<Msg>>>();

        let tab_row = w::Row::with_children(tabs).spacing(s::S4);

        let tab_content: Element<Msg> = match self.tab {
            Tab::Prompt => {
                let prompt_response_view: Element<Msg> = match &self.prompt_status {
                    PromptStatus::Ready => w::Column::new().into(),
                    PromptStatus::Response(response) => {
                        w::text(format!("Response: {}", response)).into()
                    }
                    PromptStatus::Error(err) => {
                        w::text(format!("Error: {}", err.to_nice_error().to_string())).into()
                    }
                };

                w::column![
                    w::text_input("", &self.prompt_field).on_input(Msg::PromptFieldChanged),
                    w::button("Submit").on_press(Msg::ClickedSubmitPrompt),
                    prompt_response_view,
                ]
                .into()
            }
            Tab::Reaction => {
                let reaction_response_view: Element<Msg> = match &self.reaction_status {
                    ReactionStatus::Ready => w::Column::new().into(),
                    ReactionStatus::Response(response) => {
                        w::Column::with_children(
                            response
                                .iter()
                                .map(|action| w::text(format!("Action: {:#?}", action)).into())
                                .collect::<Vec<_>>(),
                        )
                        .into()
                        // w::text(format!("Response: {}", response)).into()
                    }
                    ReactionStatus::Error(err) => {
                        w::text(format!("Error: {}", err.to_nice_error().to_string())).into()
                    }
                };

                w::column![
                    w::text("Identity"),
                    w::text_input("Identity", &self.identity_field)
                        .on_input(Msg::IdentityFieldChanged),
                    w::text("Memories"),
                    w::Column::with_children(memories_children).spacing(s::S4),
                    w::button("Add Memory").on_press(Msg::ClickedAddMemory),
                    w::text("Situation"),
                    w::text_input("Situation", &self.situation_field)
                        .on_input(|field| { Msg::SituationFieldChanged(field) }),
                    w::text("State of Mind"),
                    w::text_input("State of Mind", &self.state_of_mind_field)
                        .on_input(|field| { Msg::StateOfMindFieldChanged(field) }),
                    w::button("Submit Reaction").on_press(Msg::ClickedSubmitReaction),
                    reaction_response_view,
                ]
                .spacing(s::S4)
                .into()
            }
            Tab::Identity => self.new_identity_page.view().map(Msg::NewIdentityPageMsg),
            Tab::Person => self.new_person_page.view().map(Msg::NewPersonPageMsg),
            Tab::Memory => self.memory_page.view().map(Msg::MemoryPageMsg),
            Tab::StateOfMind => self.state_of_mind_page.view().map(Msg::StateOfMindPageMsg),
            Tab::Scene => self.scene_page.view().map(Msg::SceneMsg),
            Tab::Job => self.job_page.view().map(Msg::JobPageMsg),
        };

        let scrollable_content = w::scrollable(tab_content);

        w::container(w::column![tab_row, scrollable_content].spacing(s::S4))
            .padding(s::S4)
            .into()
    }

    fn theme(&self) -> Theme {
        fn from_ints(r: u8, g: u8, b: u8) -> Color {
            Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
        }

        Theme::custom(
            "arizona2".to_string(),
            iced::theme::Palette {
                background: from_ints(3, 9, 7),
                text: from_ints(176, 166, 154),
                primary: from_ints(227, 211, 75),
                success: from_ints(10, 202, 26),
                danger: from_ints(242, 29, 35),
            },
        )
    }
}

pub async fn run() -> Result<(), Error> {
    let flags = Flags::get().await?;

    let iced_result = iced::application(Model::title, Model::update, Model::view)
        .theme(Model::theme)
        .run_with(move || Model::new(flags));

    iced_result.map_err(Error::IcedRunError)
}
