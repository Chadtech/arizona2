mod style;

use self::style as s;
use crate::nice_display::NiceDisplay;
use crate::open_ai_key::OpenAiKey;
use crate::worker::Worker;
use crate::{open_ai, worker};
use iced;
use iced::{widget as w, Application, Color, Element, Font, Task, Theme};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

const STORAGE_FILE_PATH: &str = "storage.json";

#[derive(Default)]
enum State {
    #[default]
    Loading,
    Loaded(Model),
    // TODO maybe a special init fail error type?
    FailedToLoad(Error),
}

#[derive(Debug)]
enum SuperMsg {
    LoadedFlags(Result<Flags, Error>),
    LoadedMsg(Msg),
}

impl State {
    fn new() -> (Self, Task<SuperMsg>) {
        (State::Loading, Self::get_flags())
    }

    fn title(&self) -> String {
        "Arizona 2 Admin".to_string()
    }

    fn get_flags() -> Task<SuperMsg> {
        println!("Getting flags...");
        Task::perform(
            async {
                let worker = Worker::new().map_err(Error::WorkerInitError)?;

                let storage = Storage::read_from_file_system()?;

                Ok(Flags { worker, storage })
            },
            SuperMsg::LoadedFlags,
        )
    }

    fn update(&mut self, message: SuperMsg) -> Task<SuperMsg> {
        match message {
            SuperMsg::LoadedFlags(result) => match result {
                Ok(flags) => {
                    let (model, task) = Model::new(flags);
                    *self = State::Loaded(model);
                    Task::none()
                }
                Err(err) => {
                    *self = State::FailedToLoad(err);
                    Task::none()
                }
            },
            SuperMsg::LoadedMsg(msg) => match self {
                State::Loaded(model) => model.update(msg).map(|m| SuperMsg::LoadedMsg(m)),
                _ => Task::none(),
            },
        }
    }

    fn view(&self) -> Element<SuperMsg> {
        match self {
            State::Loading => w::text("Loading...").into(),
            State::Loaded(model) => Model::view(model).map(|msg| SuperMsg::LoadedMsg(msg)),
            State::FailedToLoad(err) => w::text(format!("Error: {}", err.to_nice_error())).into(),
        }
    }
}

struct Model {
    prompt_field: String,
    identity_field: String,
    prompt_response: PromptResponse,
    memory_fields: Vec<w::text_editor::Content>,
    worker: Worker,
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
        }
    }
}

enum PromptResponse {
    Ready,
    Response(String),
    Error(open_ai::CompletionError),
}

#[derive(Serialize, Deserialize, Debug)]
struct Storage {
    prompt: String,
    #[serde(default)]
    memories: Vec<String>,
    #[serde(default)]
    identity_field: String,
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
        }
    }
}

#[derive(Debug)]
struct Flags {
    worker: Worker,
    storage: Storage,
}

#[derive(Debug, Clone)]
enum Msg {
    PromptFieldChanged(String),
    ClickedSubmitPrompt,
    ClickedAddMemory,
    SubmissionResult(Result<String, open_ai::CompletionError>),
    MemoryUpdated {
        index: usize,
        action: w::text_editor::Action,
    },
    IdentityFieldChanged(String),
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
    // type Executor = iced::executor::Default;
    // type Message = Msg;
    // type Theme = Theme;
    // type Flags = Flags;

    fn new(flags: Flags) -> (Self, Task<Msg>) {
        let model = Model {
            prompt_field: flags.storage.prompt,
            identity_field: flags.storage.identity_field,
            prompt_response: PromptResponse::Ready,
            memory_fields: flags
                .storage
                .memories
                .iter()
                .map(|content_str| w::text_editor::Content::with_text(content_str))
                .collect(),
            worker: flags.worker,
            error: None,
        };

        (model, Task::none())
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
                    submit_prompt(open_ai_key, reqwest_client, self.prompt_field.clone()),
                    Msg::SubmissionResult,
                )
            }
            Msg::SubmissionResult(result) => {
                self.prompt_response = match result {
                    Ok(response) => PromptResponse::Response(response.clone()),
                    Err(err) => PromptResponse::Error(err.clone()),
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
        }
    }

    fn view(&self) -> Element<Msg> {
        let prompt_response_view: Element<Msg> = match &self.prompt_response {
            PromptResponse::Ready => w::Column::new().into(),
            PromptResponse::Response(response) => w::text(format!("Response: {}", response)).into(),
            PromptResponse::Error(err) => {
                w::text(format!("Error: {}", err.to_nice_error().to_string())).into()
            }
        };

        let mut memories_children: Vec<Element<_>> = vec![];

        for (i, memory) in self.memory_fields.iter().enumerate() {
            let memories_editor = w::text_editor(memory).on_action(move |act| Msg::MemoryUpdated {
                index: i,
                action: act,
            });

            memories_children.push(memories_editor.into());
        }

        w::container(
            w::column![
                "Prompt",
                w::text_input("", &self.prompt_field).on_input(Msg::PromptFieldChanged),
                "Identity",
                w::text_input("", &self.identity_field).on_input(Msg::IdentityFieldChanged),
                prompt_response_view,
                "Memories",
                w::Column::with_children(memories_children).spacing(s::S4),
                w::button("Add Memory").on_press(Msg::ClickedAddMemory),
                w::button("Submit").on_press(Msg::ClickedSubmitPrompt)
            ]
            .spacing(s::S4),
        )
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

async fn submit_prompt(
    open_ai_key: OpenAiKey,
    client: reqwest::Client,
    prompt: String,
) -> Result<String, open_ai::CompletionError> {
    let response = open_ai::Completion::new(open_ai::Model::Gpt4p1)
        .add_message(open_ai::Role::User, prompt.as_str())
        .send_request(&open_ai_key, client)
        .await?;

    Ok(response)
}

pub async fn run() -> Result<(), Error> {
    // let worker = Worker::new().map_err(Error::WorkerInitError)?;
    //
    // let storage = Storage::read_from_file_system()?;
    //
    // let flags = Flags { worker, storage };
    //
    // let mut settings = iced::Settings::with_flags(flags);
    //
    // settings.default_font = Font::with_name("Fira Code");
    //
    // Model::run(settings).map_err(Error::IcedRunError)
    // let iced_result = iced::application(Model::new, Model::update, Model::view)
    //     // .title("Arizona 2 Admin")
    //     .run();

    let iced_result =
        iced::application(State::title, State::update, State::view).run_with(State::new);

    iced_result.map_err(Error::IcedRunError)
}
