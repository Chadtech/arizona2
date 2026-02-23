use super::style as s;
use iced::{widget as w, Element};
use serde::{Deserialize, Serialize};

pub struct Model {
    system_prompt: w::text_editor::Content,
    user_prompt: w::text_editor::Content,
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
}

impl Model {
    pub fn new(storage: &Storage) -> Self {
        Self {
            system_prompt: w::text_editor::Content::with_text(&storage.system_prompt),
            user_prompt: w::text_editor::Content::with_text(&storage.user_prompt),
        }
    }

    pub fn to_storage(&self) -> Storage {
        Storage {
            system_prompt: self.system_prompt.text(),
            user_prompt: self.user_prompt.text(),
        }
    }

    pub fn update(&mut self, msg: Msg) {
        match msg {
            Msg::SystemPromptUpdated(action) => {
                self.system_prompt.perform(action);
            }
            Msg::UserPromptUpdated(action) => {
                self.user_prompt.perform(action);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Msg> {
        let system_editor = w::text_editor(&self.system_prompt)
            .on_action(Msg::SystemPromptUpdated)
            .height(iced::Length::Fixed(180.0));

        let user_editor = w::text_editor(&self.user_prompt)
            .on_action(Msg::UserPromptUpdated)
            .height(iced::Length::Fixed(180.0));

        w::column![
            w::text("Prompt Lab"),
            w::text("System prompt").size(s::S3),
            system_editor,
            w::text("User prompt").size(s::S3),
            user_editor,
        ]
        .spacing(s::S2)
        .into()
    }
}
