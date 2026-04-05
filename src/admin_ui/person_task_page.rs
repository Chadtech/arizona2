use crate::admin_ui::s;
use crate::capability::person::PersonCapability;
use crate::capability::person_task::{NewPersonTask, PersonTaskCapability};
use crate::domain::person_name::PersonName;
use crate::domain::person_task::PersonTask;
use crate::domain::person_task_uuid::PersonTaskUuid;
use crate::domain::person_uuid::PersonUuid;
use crate::worker::Worker;
use iced::{widget as w, Element, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    person_name_input: String,
    load_status: LoadStatus,
    content_input: String,
    success_condition_input: String,
    abandon_condition_input: String,
    failure_condition_input: String,
    priority_input: String,
    create_status: CreateStatus,
}

enum LoadStatus {
    Ready,
    Loading,
    Loaded {
        person_uuid: PersonUuid,
        current_task: Option<PersonTask>,
    },
    Error(String),
}

enum CreateStatus {
    Ready,
    Creating,
    Done(PersonTaskUuid),
    Error(String),
}

#[derive(Debug, Clone)]
pub enum Msg {
    PersonNameChanged(String),
    ClickedLoadPerson,
    PersonTaskDataLoaded(Result<(PersonUuid, Option<PersonTask>), String>),
    ContentChanged(String),
    SuccessConditionChanged(String),
    AbandonConditionChanged(String),
    FailureConditionChanged(String),
    PriorityChanged(String),
    ClickedCreateTask,
    TaskCreated(Result<PersonTaskUuid, String>),
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Storage {
    #[serde(default)]
    person_name_input: String,
    #[serde(default)]
    content_input: String,
    #[serde(default)]
    success_condition_input: String,
    #[serde(default)]
    abandon_condition_input: String,
    #[serde(default)]
    failure_condition_input: String,
    #[serde(default)]
    priority_input: String,
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            person_name_input: storage.person_name_input.clone(),
            load_status: LoadStatus::Ready,
            content_input: storage.content_input.clone(),
            success_condition_input: storage.success_condition_input.clone(),
            abandon_condition_input: storage.abandon_condition_input.clone(),
            failure_condition_input: storage.failure_condition_input.clone(),
            priority_input: storage.priority_input.clone(),
            create_status: CreateStatus::Ready,
        }
    }

    pub fn to_storage(&self) -> Storage {
        Storage {
            person_name_input: self.person_name_input.clone(),
            content_input: self.content_input.clone(),
            success_condition_input: self.success_condition_input.clone(),
            abandon_condition_input: self.abandon_condition_input.clone(),
            failure_condition_input: self.failure_condition_input.clone(),
            priority_input: self.priority_input.clone(),
        }
    }

    pub fn update(&mut self, worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::PersonNameChanged(value) => {
                self.person_name_input = value;
                self.load_status = LoadStatus::Ready;
                self.create_status = CreateStatus::Ready;
                Task::none()
            }
            Msg::ClickedLoadPerson => {
                let person_name = self.person_name_input.trim().to_string();
                if person_name.is_empty() {
                    self.load_status = LoadStatus::Error("Person name cannot be empty".to_string());
                    return Task::none();
                }

                self.load_status = LoadStatus::Loading;
                Task::perform(
                    async move { load_person_task_data(&worker, person_name).await },
                    Msg::PersonTaskDataLoaded,
                )
            }
            Msg::PersonTaskDataLoaded(result) => {
                self.load_status = match result {
                    Ok((person_uuid, current_task)) => LoadStatus::Loaded {
                        person_uuid,
                        current_task,
                    },
                    Err(err) => LoadStatus::Error(err),
                };
                Task::none()
            }
            Msg::ContentChanged(value) => {
                self.content_input = value;
                self.create_status = CreateStatus::Ready;
                Task::none()
            }
            Msg::SuccessConditionChanged(value) => {
                self.success_condition_input = value;
                self.create_status = CreateStatus::Ready;
                Task::none()
            }
            Msg::AbandonConditionChanged(value) => {
                self.abandon_condition_input = value;
                self.create_status = CreateStatus::Ready;
                Task::none()
            }
            Msg::FailureConditionChanged(value) => {
                self.failure_condition_input = value;
                self.create_status = CreateStatus::Ready;
                Task::none()
            }
            Msg::PriorityChanged(value) => {
                self.priority_input = value;
                self.create_status = CreateStatus::Ready;
                Task::none()
            }
            Msg::ClickedCreateTask => {
                let person_uuid = match &self.load_status {
                    LoadStatus::Loaded { person_uuid, .. } => person_uuid.clone(),
                    _ => return Task::none(),
                };

                let content = self.content_input.trim();
                if content.is_empty() {
                    self.create_status = CreateStatus::Error("Task content cannot be empty".to_string());
                    return Task::none();
                }

                let priority = match self.priority_input.trim().parse::<i32>() {
                    Ok(priority) => priority,
                    Err(_) => {
                        self.create_status = CreateStatus::Error("Priority must be a number".to_string());
                        return Task::none();
                    }
                };

                let new_person_task = NewPersonTask {
                    person_uuid,
                    content: content.to_string(),
                    success_condition: optional_string(&self.success_condition_input),
                    abandon_condition: optional_string(&self.abandon_condition_input),
                    failure_condition: optional_string(&self.failure_condition_input),
                    priority,
                };

                self.create_status = CreateStatus::Creating;
                Task::perform(
                    async move { worker.set_persons_current_active_task(new_person_task).await },
                    Msg::TaskCreated,
                )
            }
            Msg::TaskCreated(result) => match result {
                Ok(person_task_uuid) => {
                    self.create_status = CreateStatus::Done(person_task_uuid);
                    self.content_input.clear();
                    self.success_condition_input.clear();
                    self.abandon_condition_input.clear();
                    self.failure_condition_input.clear();

                    let person_name = self.person_name_input.clone();
                    self.load_status = LoadStatus::Loading;
                    Task::perform(
                        async move { load_person_task_data(&worker, person_name).await },
                        Msg::PersonTaskDataLoaded,
                    )
                }
                Err(err) => {
                    self.create_status = CreateStatus::Error(err);
                    Task::none()
                }
            },
        }
    }

    pub fn view(&self) -> Element<'_, Msg> {
        let load_section = w::column![
            w::text("Person Name"),
            w::row![
                w::text_input("Enter person name", &self.person_name_input)
                    .on_input(Msg::PersonNameChanged),
                w::button("Load Person").on_press(Msg::ClickedLoadPerson),
            ]
            .spacing(s::S1),
            load_status_view(&self.load_status),
        ]
        .spacing(s::S2);

        let current_task_section = current_task_view(&self.load_status);

        let create_section: Element<'_, Msg> = match &self.load_status {
            LoadStatus::Loaded { .. } => w::column![
                w::text("New Current Task"),
                w::text_input("Task content", &self.content_input)
                    .on_input(Msg::ContentChanged),
                w::text_input("Success condition (optional)", &self.success_condition_input)
                .on_input(Msg::SuccessConditionChanged),
                w::text_input("Abandon condition (optional)", &self.abandon_condition_input)
                .on_input(Msg::AbandonConditionChanged),
                w::text_input("Failure condition (optional)", &self.failure_condition_input)
                .on_input(Msg::FailureConditionChanged),
                w::text_input("Priority (0-100)", &self.priority_input)
                    .on_input(Msg::PriorityChanged),
                w::button("Create Current Task").on_press(Msg::ClickedCreateTask),
                create_status_view(&self.create_status),
            ]
            .spacing(s::S2)
            .into(),
            _ => w::column![
                w::text("New Current Task"),
                w::text("Load a person to create a task."),
            ]
            .spacing(s::S1)
            .into(),
        };

        w::column![
            w::text("Person Tasks"),
            load_section,
            current_task_section,
            create_section,
        ]
        .spacing(s::S4)
        .into()
    }
}

fn load_status_view(status: &LoadStatus) -> Element<'_, Msg> {
    match status {
        LoadStatus::Ready => w::text("Ready").into(),
        LoadStatus::Loading => w::text("Loading person task data...").into(),
        LoadStatus::Loaded { current_task, .. } => match current_task {
            Some(_) => w::text("Loaded person and current task").into(),
            None => w::text("Loaded person with no current active task").into(),
        },
        LoadStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
    }
}

fn create_status_view(status: &CreateStatus) -> Element<'_, Msg> {
    match status {
        CreateStatus::Ready => w::text("Ready").into(),
        CreateStatus::Creating => w::text("Creating task...").into(),
        CreateStatus::Done(person_task_uuid) => {
            w::text(format!("Task created: {}", person_task_uuid.to_uuid())).into()
        }
        CreateStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
    }
}

fn current_task_view(status: &LoadStatus) -> Element<'_, Msg> {
    match status {
        LoadStatus::Loaded { current_task, .. } => match current_task {
            Some(task) => {
                let created_at = task.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
                w::column![
                    w::text("Current Active Task"),
                    w::text(format!("Priority: {}", task.priority)).size(s::S3),
                    w::text(format!("Created: {}", created_at)).size(s::S3),
                    w::text(format!("UUID: {}", task.uuid.to_uuid())).size(s::S3),
                    w::text(&task.content),
                    optional_condition_view("Success", task.success_condition.as_deref()),
                    optional_condition_view("Abandon", task.abandon_condition.as_deref()),
                    optional_condition_view("Failure", task.failure_condition.as_deref()),
                ]
                .spacing(s::S1)
                .into()
            }
            None => w::column![
                w::text("Current Active Task"),
                w::text("No active task found."),
            ]
            .spacing(s::S1)
            .into(),
        },
        _ => w::text("").into(),
    }
}

fn optional_condition_view<'a>(label: &'a str, value: Option<&'a str>) -> Element<'a, Msg> {
    match value {
        Some(text) if !text.trim().is_empty() => w::text(format!("{}: {}", label, text)).into(),
        _ => w::text("").into(),
    }
}

fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

async fn load_person_task_data(
    worker: &Worker,
    person_name: String,
) -> Result<(PersonUuid, Option<PersonTask>), String> {
    let person_uuid = worker
        .get_person_uuid_by_name(PersonName::from_string(person_name))
        .await?;
    let current_task = worker.get_persons_current_active_task(&person_uuid).await?;
    Ok((person_uuid, current_task))
}
