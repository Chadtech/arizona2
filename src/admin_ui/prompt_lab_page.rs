use super::call;
use super::style as s;
use crate::nice_display::NiceDisplay;
use crate::open_ai::completion::CompletionError;
use crate::worker::Worker;
use iced::{widget as w, Element, Task};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct Model {
    system_prompt: w::text_editor::Content,
    user_prompt: w::text_editor::Content,
    status: Status,
}

enum Status {
    Ready,
    Submitting,
    Response(String),
    Error(CompletionError),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Storage {
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub user_prompt: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    SystemPromptUpdated(w::text_editor::Action),
    UserPromptUpdated(w::text_editor::Action),
    ClickedSubmit,
    SubmissionResult(Result<String, CompletionError>),
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            system_prompt: w::text_editor::Content::with_text(&storage.system_prompt),
            user_prompt: w::text_editor::Content::with_text(&storage.user_prompt),
            status: Status::Ready,
        }
    }

    pub fn to_storage(&self) -> Storage {
        Storage {
            system_prompt: self.system_prompt.text(),
            user_prompt: self.user_prompt.text(),
        }
    }

    pub fn update(&mut self, worker: Arc<Worker>, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::SystemPromptUpdated(action) => {
                self.system_prompt.perform(action);
                Task::none()
            }
            Msg::UserPromptUpdated(action) => {
                self.user_prompt.perform(action);
                Task::none()
            }
            Msg::ClickedSubmit => {
                let system_prompt = self.system_prompt.text();
                let user_prompt = self.user_prompt.text();
                let open_ai_key = worker.open_ai_key.clone();
                self.status = Status::Submitting;

                Task::perform(
                    call::submit_prompt_lab(open_ai_key, system_prompt, user_prompt),
                    Msg::SubmissionResult,
                )
            }
            Msg::SubmissionResult(result) => {
                self.status = match result {
                    Ok(response) => Status::Response(response),
                    Err(err) => Status::Error(err),
                };
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Msg> {
        let system_editor = w::text_editor(&self.system_prompt)
            .on_action(Msg::SystemPromptUpdated)
            .height(iced::Length::Fixed(180.0));

        let user_editor = w::text_editor(&self.user_prompt)
            .on_action(Msg::UserPromptUpdated)
            .height(iced::Length::Fixed(420.0));

        let response_view: Element<'_, Msg> = match &self.status {
            Status::Ready => w::text("").into(),
            Status::Submitting => w::text("Submitting...").into(),
            Status::Response(text) => w::text(text).into(),
            Status::Error(err) => w::text(format!("Error: {}", err.message())).into(),
        };

        let submit_button = match self.status {
            Status::Submitting => w::button("Submitting..."),
            _ => w::button("Submit").on_press(Msg::ClickedSubmit),
        };

        w::column![
            w::text("Prompt Lab"),
            w::text("System prompt").size(s::S3),
            system_editor,
            w::text("User prompt").size(s::S3),
            user_editor,
            submit_button,
            response_view,
        ]
        .spacing(s::S2)
        .into()
    }
}
