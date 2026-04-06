mod call;
mod job_page;
mod memory_page;
mod messages_page;
mod motivation_page;
mod new_identity_page;
mod person_page;
mod person_task_page;
mod prompt_lab_page;
mod reaction_page;
mod scene_page;
mod state_of_mind_page;
mod style;

use self::style as s;
use crate::capability::job_runner_settings::JobRunnerSettingsCapability;
use crate::domain::logger::{Level, Logger};
use crate::nice_display::NiceDisplay;
use crate::open_ai::completion::CompletionError;
use crate::worker;
use crate::worker::Worker;
use iced::{widget as w, Element, Length, Subscription, Task, Theme};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

const STORAGE_FILE_PATH: &str = "storage.json";

struct Model {
    prompt_field: String,
    prompt_status: PromptStatus,
    new_identity_page: new_identity_page::Model,
    person_page: person_page::Model,
    memory_page: memory_page::Model,
    motivation_page: motivation_page::Model,
    person_task_page: person_task_page::Model,
    messages_page: messages_page::Model,
    state_of_mind_page: state_of_mind_page::Model,
    scene_page: scene_page::Model,
    job_page: job_page::Model,
    reaction_page: reaction_page::Model,
    prompt_lab_page: prompt_lab_page::Model,
    tab: Tab,
    worker: Arc<Worker>,
    error: Option<Error>,
    job_runner_poll_interval_input: String,
    job_runner_poll_interval_status: JobRunnerPollIntervalStatus,
    job_runner_enabled: bool,
    job_runner_enabled_status: JobRunnerEnabledStatus,
}

impl Model {
    pub fn to_storage(&self) -> Storage {
        Storage {
            prompt: self.prompt_field.clone(),
            new_identity: self.new_identity_page.to_storage(),
            person: self.person_page.to_storage(),
            memory: self.memory_page.to_storage(),
            motivation: self.motivation_page.to_storage(),
            person_task: self.person_task_page.to_storage(),
            messages: self.messages_page.to_storage(),
            state_of_mind: self.state_of_mind_page.to_storage(),
            scene: self.scene_page.to_storage(),
            job: self.job_page.to_storage(),
            reaction: self.reaction_page.to_storage(),
            prompt_lab: self.prompt_lab_page.to_storage(),
            tab: self.tab,
        }
    }
}

enum PromptStatus {
    Ready,
    Response(String),
    Error(CompletionError),
}

enum JobRunnerPollIntervalStatus {
    Loading,
    Ready,
    Saving,
    Error(String),
}

enum JobRunnerEnabledStatus {
    Loading,
    Ready,
    Saving,
    Error(String),
}

#[derive(Serialize, Deserialize, Debug)]
struct Storage {
    prompt: String,
    #[serde(default)]
    tab: Tab,
    #[serde(default)]
    new_identity: new_identity_page::Storage,
    #[serde(default)]
    #[serde(alias = "new_person")]
    person: person_page::Storage,
    #[serde(default)]
    memory: memory_page::Storage,
    #[serde(default, alias = "goal")]
    motivation: motivation_page::Storage,
    #[serde(default)]
    person_task: person_task_page::Storage,
    #[serde(default)]
    messages: messages_page::Storage,
    #[serde(default)]
    state_of_mind: state_of_mind_page::Storage,
    #[serde(default)]
    scene: scene_page::Storage,
    #[serde(default)]
    job: job_page::Storage,
    #[serde(default)]
    reaction: reaction_page::Storage,
    #[serde(default)]
    prompt_lab: prompt_lab_page::Storage,
}

impl Storage {
    pub fn save_to_file_system(&self) -> Result<(), Error> {
        // Serialize the Storage struct to JSON
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| Error::StorageSerialization(e.to_string()))?;

        // Create a file to save the JSON
        let path = Path::new(STORAGE_FILE_PATH);
        let mut file = File::create(path).map_err(Error::StorageFileCreation)?;

        // Write the JSON to the file
        file.write_all(json.as_bytes())
            .map_err(Error::StorageFileWrite)?;

        Ok(())
    }

    pub fn read_from_file_system() -> Result<Self, Error> {
        // Check if the file exists
        let path = Path::new(STORAGE_FILE_PATH);
        if !path.exists() {
            return Ok(Self::default());
        }

        // Open the file
        let file = File::open(path).map_err(Error::StorageFileRead)?;

        // Deserialize the JSON into a Storage struct
        let storage: Storage = serde_json::from_reader(file)
            .map_err(|e| Error::StorageDeserialization(e.to_string()))?;

        Ok(storage)
    }

    pub fn default() -> Self {
        Storage {
            prompt: String::new(),
            tab: Tab::default(),
            new_identity: new_identity_page::Storage::default(),
            person: person_page::Storage::default(),
            memory: memory_page::Storage::default(),
            motivation: motivation_page::Storage::default(),
            person_task: person_task_page::Storage::default(),
            messages: messages_page::Storage::default(),
            state_of_mind: state_of_mind_page::Storage::default(),
            scene: scene_page::Storage::default(),
            job: job_page::Storage::default(),
            reaction: reaction_page::Storage::default(),
            prompt_lab: prompt_lab_page::Storage::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq, Default)]
enum Tab {
    #[default]
    Prompt,
    PromptLab,
    Reaction,
    Identity,
    Person,
    Memory,
    #[serde(alias = "Goal")]
    Motivation,
    PersonTask,
    Messages,
    StateOfMind,
    Scene,
    Job,
}

impl Tab {
    pub fn to_label(self) -> String {
        match self {
            Tab::Prompt => "Prompt".to_string(),
            Tab::PromptLab => "Prompt Lab".to_string(),
            Tab::Reaction => "Reaction".to_string(),
            Tab::Identity => "Identity".to_string(),
            Tab::Person => "Person".to_string(),
            Tab::Memory => "Memory".to_string(),
            Tab::Motivation => "Motivation".to_string(),
            Tab::PersonTask => "Person Task".to_string(),
            Tab::Messages => "Messages".to_string(),
            Tab::StateOfMind => "State of Mind".to_string(),
            Tab::Scene => "Scene".to_string(),
            Tab::Job => "Job".to_string(),
        }
    }

    pub fn all() -> Vec<Tab> {
        vec![
            Tab::Messages,
            Tab::Job,
            Tab::Prompt,
            Tab::PromptLab,
            Tab::Reaction,
            Tab::Identity,
            Tab::Person,
            Tab::Memory,
            Tab::Motivation,
            Tab::PersonTask,
            Tab::StateOfMind,
            Tab::Scene,
        ]
    }

    pub fn init_task(self, worker: &Arc<Worker>) -> Task<Msg> {
        if self == Tab::Job {
            Task::perform(
                job_page::get_jobs(worker.clone(), job_page::initial_jobs_limit()),
                Msg::JobPage,
            )
        } else {
            Task::none()
        }
    }
}

#[derive(Debug)]
struct Flags {
    worker: Worker,
    storage: Storage,
}

impl Flags {
    async fn get() -> Result<Self, Error> {
        let logger = Logger::init(Level::Warning);

        let worker = Worker::new(logger).await.map_err(Error::WorkerInit)?;

        let storage = Storage::read_from_file_system()?;

        Ok(Flags { worker, storage })
    }
}

#[derive(Debug, Clone)]
enum Msg {
    PromptFieldChanged(String),
    ClickedSubmitPrompt,
    SubmissionResult(Result<String, CompletionError>),
    TabSelected(Tab),
    NewIdentityPage(new_identity_page::Msg),
    PersonPage(person_page::Msg),
    MemoryPage(memory_page::Msg),
    MotivationPage(motivation_page::Msg),
    PersonTaskPage(person_task_page::Msg),
    MessagesPage(messages_page::Msg),
    StateOfMindPage(state_of_mind_page::Msg),
    ScenePage(scene_page::Msg),
    JobPage(job_page::Msg),
    ReactionPage(reaction_page::Msg),
    PromptLab(prompt_lab_page::Msg),
    WarmedUpDb,
    JobRunnerPollIntervalLoaded(Result<u64, String>),
    JobRunnerPollIntervalInputChanged(String),
    JobRunnerPollIntervalSubmitted,
    JobRunnerPollIntervalSaved(Result<(), String>),
    JobRunnerEnabledLoaded(Result<bool, String>),
    JobRunnerEnabledToggled,
    JobRunnerEnabledSaved(Result<(), String>),
}

#[derive(Debug)]
pub enum Error {
    IcedRun(iced::Error),
    WorkerInit(worker::InitError),
    StorageFileCreation(std::io::Error),
    StorageFileWrite(std::io::Error),
    StorageSerialization(String),
    StorageFileRead(std::io::Error),
    StorageDeserialization(String),
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::IcedRun(err) => format!("Iced run error: {}", err),
            Error::WorkerInit(err) => err.message(),
            Error::StorageFileCreation(err) => format!("Storage file creation error: {}", err),
            Error::StorageFileWrite(err) => format!("Storage file write error: {}", err),
            Error::StorageSerialization(msg) => {
                format!("Storage serialization error: {}", msg)
            }
            Error::StorageFileRead(err) => format!("Storage file read error: {}", err),
            Error::StorageDeserialization(msg) => {
                format!("Storage deserialization error: {}", msg)
            }
        }
    }
}

impl Model {
    fn new(flags: Flags) -> (Self, Task<Msg>) {
        let tab = flags.storage.tab;

        let mut model = Model {
            prompt_field: flags.storage.prompt,
            prompt_status: PromptStatus::Ready,
            new_identity_page: new_identity_page::Model::new(&flags.storage.new_identity),
            person_page: person_page::Model::new(&flags.storage.person),
            memory_page: memory_page::Model::new(&flags.storage.memory),
            motivation_page: motivation_page::Model::new(&flags.storage.motivation),
            person_task_page: person_task_page::Model::new(&flags.storage.person_task),
            messages_page: messages_page::Model::new(&flags.storage.messages),
            scene_page: scene_page::Model::new(&flags.storage.scene),
            job_page: job_page::Model::new(&flags.storage.job),
            reaction_page: reaction_page::Model::new(&flags.storage.reaction),
            prompt_lab_page: prompt_lab_page::Model::new(&flags.storage.prompt_lab),
            tab,
            worker: Arc::new(flags.worker),
            error: None,
            state_of_mind_page: state_of_mind_page::Model::new(&flags.storage.state_of_mind),
            job_runner_poll_interval_input: String::new(),
            job_runner_poll_interval_status: JobRunnerPollIntervalStatus::Loading,
            job_runner_enabled: true,
            job_runner_enabled_status: JobRunnerEnabledStatus::Loading,
        };

        let worker2 = model.worker.clone();
        let worker3 = model.worker.clone();
        let worker4 = model.worker.clone();

        let tab_task = tab.init_task(&model.worker);
        let messages_tab_task = if tab == Tab::Messages {
            model
                .messages_page
                .on_tab_activated(model.worker.clone())
                .map(Msg::MessagesPage)
        } else {
            Task::none()
        };
        let scene_tab_task = if tab == Tab::Scene {
            model
                .scene_page
                .on_tab_activated(model.worker.clone())
                .map(Msg::ScenePage)
        } else {
            Task::none()
        };

        (
            model,
            Task::batch(vec![
                Task::perform(async move { worker2.warm_up_db_connection().await }, |_| {
                    Msg::WarmedUpDb
                }),
                Task::perform(
                    async move { worker3.get_job_runner_poll_interval_secs().await },
                    Msg::JobRunnerPollIntervalLoaded,
                ),
                Task::perform(
                    async move { worker4.get_job_runner_enabled().await },
                    Msg::JobRunnerEnabledLoaded,
                ),
                tab_task,
                messages_tab_task,
                scene_tab_task,
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
            Msg::TabSelected(tab) => {
                self.tab = tab;

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                let init_task = self.tab.init_task(&self.worker);
                let tab_task = match self.tab {
                    Tab::Messages => self
                        .messages_page
                        .on_tab_activated(self.worker.clone())
                        .map(Msg::MessagesPage),
                    Tab::Scene => self
                        .scene_page
                        .on_tab_activated(self.worker.clone())
                        .map(Msg::ScenePage),
                    _ => Task::none(),
                };
                Task::batch(vec![init_task, tab_task])
            }
            Msg::NewIdentityPage(sub_msg) => {
                let task = self.new_identity_page.update(self.worker.clone(), sub_msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::NewIdentityPage)
            }
            Msg::PersonPage(sub_msg) => {
                let task = self.person_page.update(self.worker.clone(), sub_msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::PersonPage)
            }
            Msg::MemoryPage(sub_msg) => {
                let task = self.memory_page.update(self.worker.clone(), sub_msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::MemoryPage)
            }
            Msg::MotivationPage(sub_msg) => {
                let task = self.motivation_page.update(self.worker.clone(), sub_msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::MotivationPage)
            }
            Msg::PersonTaskPage(sub_msg) => {
                let task = self.person_task_page.update(self.worker.clone(), sub_msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::PersonTaskPage)
            }
            Msg::MessagesPage(sub_msg) => {
                let task = self.messages_page.update(self.worker.clone(), sub_msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::MessagesPage)
            }
            Msg::WarmedUpDb => Task::none(),
            Msg::JobRunnerPollIntervalLoaded(result) => {
                match result {
                    Ok(secs) => {
                        self.job_runner_poll_interval_input = secs.to_string();
                        self.job_runner_poll_interval_status = JobRunnerPollIntervalStatus::Ready;
                    }
                    Err(err) => {
                        self.job_runner_poll_interval_status =
                            JobRunnerPollIntervalStatus::Error(err);
                    }
                }
                Task::none()
            }
            Msg::JobRunnerPollIntervalInputChanged(value) => {
                self.job_runner_poll_interval_input = value;
                if matches!(
                    self.job_runner_poll_interval_status,
                    JobRunnerPollIntervalStatus::Error(_)
                ) {
                    self.job_runner_poll_interval_status = JobRunnerPollIntervalStatus::Ready;
                }
                Task::none()
            }
            Msg::JobRunnerPollIntervalSubmitted => {
                let secs_res = self
                    .job_runner_poll_interval_input
                    .trim()
                    .parse::<u64>()
                    .map_err(|_| "Enter a whole number of seconds".to_string());

                let secs = match secs_res {
                    Ok(secs) => secs,
                    Err(err) => {
                        self.job_runner_poll_interval_status =
                            JobRunnerPollIntervalStatus::Error(err);
                        return Task::none();
                    }
                };

                let worker = self.worker.clone();
                self.job_runner_poll_interval_status = JobRunnerPollIntervalStatus::Saving;
                Task::perform(
                    async move { worker.set_job_runner_poll_interval_secs(secs).await },
                    Msg::JobRunnerPollIntervalSaved,
                )
            }
            Msg::JobRunnerPollIntervalSaved(result) => {
                match result {
                    Ok(()) => {
                        self.job_runner_poll_interval_status = JobRunnerPollIntervalStatus::Ready;
                    }
                    Err(err) => {
                        self.job_runner_poll_interval_status =
                            JobRunnerPollIntervalStatus::Error(err);
                    }
                }
                Task::none()
            }
            Msg::JobRunnerEnabledLoaded(result) => {
                match result {
                    Ok(enabled) => {
                        self.job_runner_enabled = enabled;
                        self.job_runner_enabled_status = JobRunnerEnabledStatus::Ready;
                    }
                    Err(err) => {
                        self.job_runner_enabled_status = JobRunnerEnabledStatus::Error(err);
                    }
                }
                Task::none()
            }
            Msg::JobRunnerEnabledToggled => {
                let new_value = !self.job_runner_enabled;
                self.job_runner_enabled = new_value;
                let worker = self.worker.clone();
                self.job_runner_enabled_status = JobRunnerEnabledStatus::Saving;
                Task::perform(
                    async move { worker.set_job_runner_enabled(new_value).await },
                    Msg::JobRunnerEnabledSaved,
                )
            }
            Msg::JobRunnerEnabledSaved(result) => {
                match result {
                    Ok(()) => {
                        self.job_runner_enabled_status = JobRunnerEnabledStatus::Ready;
                    }
                    Err(err) => {
                        self.job_runner_enabled_status = JobRunnerEnabledStatus::Error(err);
                    }
                }
                Task::none()
            }
            Msg::StateOfMindPage(msg) => {
                let task = self.state_of_mind_page.update(self.worker.clone(), msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::StateOfMindPage)
            }
            Msg::ScenePage(sub_msg) => {
                let task = self.scene_page.update(self.worker.clone(), sub_msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::ScenePage)
            }
            Msg::JobPage(sub_msg) => {
                let task = self.job_page.update(self.worker.clone(), sub_msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::JobPage)
            }
            Msg::ReactionPage(sub_msg) => {
                let task = self.reaction_page.update(self.worker.clone(), sub_msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::ReactionPage)
            }
            Msg::PromptLab(sub_msg) => {
                let task = self.prompt_lab_page.update(self.worker.clone(), sub_msg);

                if let Err(err) = self.to_storage().save_to_file_system() {
                    self.error = Some(err);
                }

                task.map(Msg::PromptLab)
            }
        }
    }

    fn view(&self) -> Element<'_, Msg> {
        let tabs = Tab::all()
            .iter()
            .flat_map(|tab: &Tab| {
                let radio_tab: Element<Msg> =
                    w::radio(tab.to_label(), *tab, Some(self.tab), Msg::TabSelected).into();

                if *tab == Tab::Job {
                    vec![radio_tab, w::horizontal_rule(1).into()]
                } else {
                    vec![radio_tab]
                }
            })
            .collect::<Vec<Element<Msg>>>();

        let tab_column = w::Column::with_children(tabs)
            .spacing(s::S2)
            .width(Length::Fixed(180.0));

        let time_controls = w::row![
            w::text("Job interval (s)"),
            w::text_input("", &self.job_runner_poll_interval_input)
                .on_input(Msg::JobRunnerPollIntervalInputChanged)
                .on_submit(Msg::JobRunnerPollIntervalSubmitted)
                .width(Length::Fixed(120.0)),
            w::button("Save").on_press(Msg::JobRunnerPollIntervalSubmitted),
            view_poll_interval_status(&self.job_runner_poll_interval_status),
            w::button(job_runner_enabled_label(self.job_runner_enabled))
                .on_press(Msg::JobRunnerEnabledToggled),
            view_enabled_status(&self.job_runner_enabled_status),
        ]
        .spacing(s::S4);

        let tab_content: Element<Msg> = match self.tab {
            Tab::Prompt => {
                let prompt_response_view: Element<Msg> = match &self.prompt_status {
                    PromptStatus::Ready => w::Column::new().into(),
                    PromptStatus::Response(response) => {
                        w::text(format!("Response: {}", response)).into()
                    }
                    PromptStatus::Error(err) => {
                        w::text(format!("Error: {}", err.to_nice_error())).into()
                    }
                };

                w::column![
                    w::text_input("", &self.prompt_field).on_input(Msg::PromptFieldChanged),
                    w::button("Submit").on_press(Msg::ClickedSubmitPrompt),
                    prompt_response_view,
                ]
                .into()
            }
            Tab::PromptLab => self.prompt_lab_page.view().map(Msg::PromptLab),
            Tab::Reaction => self.reaction_page.view().map(Msg::ReactionPage),
            Tab::Identity => self.new_identity_page.view().map(Msg::NewIdentityPage),
            Tab::Person => self.person_page.view().map(Msg::PersonPage),
            Tab::Memory => self.memory_page.view().map(Msg::MemoryPage),
            Tab::Motivation => self.motivation_page.view().map(Msg::MotivationPage),
            Tab::PersonTask => self.person_task_page.view().map(Msg::PersonTaskPage),
            Tab::Messages => self.messages_page.view().map(Msg::MessagesPage),
            Tab::StateOfMind => self.state_of_mind_page.view().map(Msg::StateOfMindPage),
            Tab::Scene => self.scene_page.view().map(Msg::ScenePage),
            Tab::Job => self.job_page.view().map(Msg::JobPage),
        };

        let scrollable_content = w::scrollable(tab_content);
        let main_content = w::column![time_controls, scrollable_content].spacing(s::S4);

        w::container(w::row![tab_column, main_content].spacing(s::S4))
            .padding(s::S4)
            .into()
    }

    fn subscription(&self) -> Subscription<Msg> {
        let mut subs = Vec::new();

        if self.tab == Tab::Job {
            subs.push(self.job_page.subscription().map(Msg::JobPage));
        }

        if self.tab == Tab::Messages {
            subs.push(self.messages_page.subscription().map(Msg::MessagesPage));
        }

        Subscription::batch(subs)
    }

    fn theme(&self) -> Theme {
        Theme::custom(
            "arizona2".to_string(),
            iced::theme::Palette {
                background: s::GRAY_VERY_DEEP,
                text: s::GRAY_VERY_SOFT,
                primary: s::GOLD_SOFT,
                success: s::GREEN_SOFT,
                danger: s::RED_SOFT,
            },
        )
    }
}

fn view_poll_interval_status(status: &JobRunnerPollIntervalStatus) -> Element<'_, Msg> {
    match status {
        JobRunnerPollIntervalStatus::Loading => w::text("Loading...").into(),
        JobRunnerPollIntervalStatus::Saving => w::text("Saving...").into(),
        JobRunnerPollIntervalStatus::Ready => w::text("").into(),
        JobRunnerPollIntervalStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
    }
}

fn job_runner_enabled_label<'a>(enabled: bool) -> &'a str {
    if enabled {
        "Job runner: On"
    } else {
        "Job runner: Off"
    }
}

fn view_enabled_status(status: &JobRunnerEnabledStatus) -> Element<'_, Msg> {
    match status {
        JobRunnerEnabledStatus::Loading => w::text("Loading...").into(),
        JobRunnerEnabledStatus::Saving => w::text("Saving...").into(),
        JobRunnerEnabledStatus::Ready => w::text("").into(),
        JobRunnerEnabledStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
    }
}

pub async fn run() -> Result<(), Error> {
    let flags = Flags::get().await?;

    let iced_result = iced::application(Model::title, Model::update, Model::view)
        .theme(Model::theme)
        .subscription(Model::subscription)
        .run_with(move || Model::new(flags));

    iced_result.map_err(Error::IcedRun)
}
