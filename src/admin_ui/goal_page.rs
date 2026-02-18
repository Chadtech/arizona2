use crate::admin_ui::s;
use crate::capability::goal::{GoalCapability, NewGoal};
use crate::capability::person::PersonCapability;
use crate::domain::goal::Goal;
use crate::domain::goal_uuid::GoalUuid;
use crate::domain::person_name::PersonName;
use crate::domain::person_uuid::PersonUuid;
use crate::worker::Worker;
use iced::{widget as w, Element, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    person_name_input: String,
    goal_content_input: String,
    goal_priority_input: String,
    load_status: LoadStatus,
    create_status: CreateStatus,
}

enum LoadStatus {
    Ready,
    Loading,
    Loaded { person_uuid: PersonUuid, goals: Vec<Goal> },
    Error(String),
}

enum CreateStatus {
    Ready,
    Creating,
    Done,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum Msg {
    PersonNameChanged(String),
    GoalContentChanged(String),
    GoalPriorityChanged(String),
    ClickedLoadGoals,
    GoalsLoaded(Result<(PersonUuid, Vec<Goal>), String>),
    ClickedCreateGoal,
    GoalCreated(Result<GoalUuid, String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Storage {
    #[serde(default)]
    person_name_input: String,
    #[serde(default)]
    goal_content_input: String,
    #[serde(default)]
    goal_priority_input: String,
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            person_name_input: String::new(),
            goal_content_input: String::new(),
            goal_priority_input: String::new(),
        }
    }
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            person_name_input: storage.person_name_input.clone(),
            goal_content_input: storage.goal_content_input.clone(),
            goal_priority_input: storage.goal_priority_input.clone(),
            load_status: LoadStatus::Ready,
            create_status: CreateStatus::Ready,
        }
    }

    pub fn to_storage(&self) -> Storage {
        Storage {
            person_name_input: self.person_name_input.clone(),
            goal_content_input: self.goal_content_input.clone(),
            goal_priority_input: self.goal_priority_input.clone(),
        }
    }

    pub fn update(&mut self, worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::PersonNameChanged(value) => {
                self.person_name_input = value;
                self.load_status = LoadStatus::Ready;
                Task::none()
            }
            Msg::GoalContentChanged(value) => {
                self.goal_content_input = value;
                Task::none()
            }
            Msg::GoalPriorityChanged(value) => {
                self.goal_priority_input = value;
                Task::none()
            }
            Msg::ClickedLoadGoals => {
                let person_name = self.person_name_input.clone();
                if person_name.trim().is_empty() {
                    self.load_status =
                        LoadStatus::Error("Person name cannot be empty".to_string());
                    return Task::none();
                }

                self.load_status = LoadStatus::Loading;
                Task::perform(
                    async move { load_goals(&worker, person_name).await },
                    Msg::GoalsLoaded,
                )
            }
            Msg::GoalsLoaded(result) => {
                self.load_status = match result {
                    Ok((person_uuid, goals)) => LoadStatus::Loaded { person_uuid, goals },
                    Err(err) => LoadStatus::Error(err),
                };
                Task::none()
            }
            Msg::ClickedCreateGoal => match &self.load_status {
                LoadStatus::Loaded { person_uuid, .. } => {
                    let priority = match self.goal_priority_input.trim().parse::<i32>() {
                        Ok(value) => value,
                        Err(_) => {
                            self.create_status = CreateStatus::Error(
                                "Priority must be a number".to_string(),
                            );
                            return Task::none();
                        }
                    };

                    let content = self.goal_content_input.trim();
                    if content.is_empty() {
                        self.create_status =
                            CreateStatus::Error("Goal content cannot be empty".to_string());
                        return Task::none();
                    }

                    self.create_status = CreateStatus::Creating;
                    let person_uuid = person_uuid.clone();
                    let content = content.to_string();
                    Task::perform(
                        async move { create_goal(&worker, person_uuid, content, priority).await },
                        Msg::GoalCreated,
                    )
                }
                _ => {
                    self.create_status =
                        CreateStatus::Error("Load a person first".to_string());
                    Task::none()
                }
            },
            Msg::GoalCreated(result) => match result {
                Ok(_) => {
                    self.create_status = CreateStatus::Done;
                    self.goal_content_input.clear();

                    let person_name = self.person_name_input.clone();
                    self.load_status = LoadStatus::Loading;
                    Task::perform(
                        async move { load_goals(&worker, person_name).await },
                        Msg::GoalsLoaded,
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
                w::button("Load Goals").on_press(Msg::ClickedLoadGoals),
            ]
            .spacing(s::S1),
            load_status_view(&self.load_status),
        ]
        .spacing(s::S2);

        let goals_list = goals_view(&self.load_status);

        let create_section = w::column![
            w::text("New Goal"),
            w::text_input("Goal content", &self.goal_content_input)
                .on_input(Msg::GoalContentChanged),
            w::text_input("Priority (integer)", &self.goal_priority_input)
                .on_input(Msg::GoalPriorityChanged),
            w::button("Create Goal").on_press(Msg::ClickedCreateGoal),
            create_status_view(&self.create_status),
        ]
        .spacing(s::S2);

        w::column![w::text("Goals"), load_section, goals_list, create_section]
            .spacing(s::S4)
            .into()
    }
}

fn load_status_view(status: &LoadStatus) -> Element<'_, Msg> {
    match status {
        LoadStatus::Ready => w::text("Ready").into(),
        LoadStatus::Loading => w::text("Loading goals...").into(),
        LoadStatus::Loaded { goals, .. } => {
            w::text(format!("Loaded {} goals", goals.len())).into()
        }
        LoadStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
    }
}

fn create_status_view(status: &CreateStatus) -> Element<'_, Msg> {
    match status {
        CreateStatus::Ready => w::text("Ready").into(),
        CreateStatus::Creating => w::text("Creating goal...").into(),
        CreateStatus::Done => w::text("Goal created").into(),
        CreateStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
    }
}

fn goals_view(status: &LoadStatus) -> Element<'_, Msg> {
    match status {
        LoadStatus::Loaded { goals, .. } => {
            if goals.is_empty() {
                return w::text("No goals found").into();
            }

            let mut col = w::column![];
            for goal in goals {
                let created_at = goal.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
                let ended_at = match goal.ended_at {
                    Some(time) => time.format("%Y-%m-%d %H:%M:%S").to_string(),
                    None => "active".to_string(),
                };

                col = col.push(
                    w::column![
                        w::text(format!("Priority: {}", goal.priority)).size(s::S3),
                        w::text(format!("Created: {}", created_at)).size(s::S3),
                        w::text(format!("Ended: {}", ended_at)).size(s::S3),
                        w::text(&goal.content),
                        w::horizontal_rule(1),
                    ]
                    .spacing(s::S1),
                );
            }

            col.spacing(s::S2).into()
        }
        LoadStatus::Loading => w::text("Loading goals...").into(),
        LoadStatus::Error(err) => w::text(format!("Error: {}", err)).into(),
        _ => w::text("").into(),
    }
}

async fn load_goals(
    worker: &Worker,
    person_name: String,
) -> Result<(PersonUuid, Vec<Goal>), String> {
    let person_uuid = worker
        .get_person_uuid_by_name(PersonName::from_string(person_name))
        .await?;
    let goals = worker.get_goals_for_person(person_uuid.clone()).await?;
    Ok((person_uuid, goals))
}

async fn create_goal(
    worker: &Worker,
    person_uuid: PersonUuid,
    content: String,
    priority: i32,
) -> Result<GoalUuid, String> {
    let new_goal = NewGoal {
        person_uuid,
        content,
        priority,
    };

    worker.create_goal(new_goal).await
}
