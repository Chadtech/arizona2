mod style;

use self::style as s;
use crate::nice_display::NiceDisplay;
use crate::open_ai_key::OpenAiKey;
use crate::worker;
use crate::worker::Worker;
use iced;
use iced::futures::future::Lazy;
use iced::widget::{Column, Row};
use iced::{widget as w, Application, Color, Command, Element, Font, Theme};
use tokio::runtime::Runtime;

struct Model {
    prompt: String,
    worker: Worker,
}

struct Flags {
    worker: Worker,
}

#[derive(Debug, Clone)]
enum Msg {
    PromptFieldChanged(String),
    CickedSubmit,
    SubmissionResult,
    SubmissionErrored,
}

pub enum Error {
    IcedRunError(iced::Error),
    OpenAiResponseError(reqwest::Error),
    WorkerInitError(worker::InitError),
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::IcedRunError(err) => format!("Iced run error: {}", err),
            Error::WorkerInitError(err) => err.message(),
            Error::OpenAiResponseError(err) => {
                format!("OpenAI response error: {}", err)
            }
        }
    }
}

impl Application for Model {
    type Executor = iced::executor::Default;
    type Message = Msg;
    type Theme = Theme;
    type Flags = Flags;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let model = Model {
            prompt: String::new(),
            worker: flags.worker,
        };

        (model, Command::none())
    }

    fn title(&self) -> String {
        "Arizona 2 Admin".to_string()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            // Msg::PressedPing => {
            //     // Command::perform(insert_beep_job(self.worker.clone()), |_result| {
            //     //     Msg::BeepInserted
            //     // })
            //     Command::none()
            // }
            Msg::PromptFieldChanged(field) => {
                self.prompt = field;
                Command::none()
            }
            Msg::CickedSubmit => {
                let open_ai_key = self.worker.open_ai_key.clone();
                let reqwest_client = self.worker.reqwest_client.clone();
                Command::perform(
                    submit_prompt(open_ai_key, reqwest_client, self.prompt.clone()),
                    |result| {
                        match result {
                            Ok(_) => Msg::SubmissionResult, // Handle success
                            Err(e) => {
                                println!("Error submitting prompt");
                                Msg::SubmissionErrored // Handle error, could be a different message
                            }
                        }
                    },
                )
            }
            Msg::SubmissionResult => Command::none(),
            Msg::SubmissionErrored => Command::none(),
        }
    }

    fn view(&self) -> Element<Msg> {
        w::container(
            w::column![
                "Prompt",
                w::text_input("", &self.prompt).on_input(Msg::PromptFieldChanged),
                w::button("Submit").on_press(Msg::CickedSubmit)
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
) -> Result<(), Error> {
    let body = serde_json::json!({
        "model": "gpt-4.1",
        "input": prompt
    });

    let res = client
        .post("https://api.openai.com/v1/responses")
        .header("Content-Type", "application/json")
        .header("Authorization", open_ai_key.to_header())
        .json(&body)
        .send()
        .await
        .map_err(Error::OpenAiResponseError)?
        .text()
        .await
        .unwrap();

    println!("Response: {}", res);

    Ok(())
}

pub async fn run() -> Result<(), Error> {
    let worker = Worker::new().map_err(Error::WorkerInitError)?;

    let flags = Flags { worker };

    let mut settings = iced::Settings::with_flags(flags);

    settings.default_font = Font::with_name("Fira Code");

    Model::run(settings).map_err(Error::IcedRunError)
}
