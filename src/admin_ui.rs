mod style;

use self::style as s;
use crate::nice_display::{NiceDisplay, NiceError};
use iced;
use iced::widget::{Column, Row};
use iced::{widget as w, Application, Color, Command, Element, Font, Theme};

struct Model {}

struct Flags {}

#[derive(Debug, Clone)]
enum Msg {
    PressedPing,
}

pub enum Error {
    IcedRunError(iced::Error),
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::IcedRunError(err) => format!("Iced run error: {}", err),
        }
    }
}

impl Application for Model {
    type Executor = iced::executor::Default;
    type Message = Msg;
    type Theme = Theme;
    type Flags = Flags;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let model = Model {};

        (model, Command::none())
    }

    fn title(&self) -> String {
        "Ahess".to_string()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Msg::PressedPing => {
                // Command::perform(insert_beep_job(self.worker.clone()), |_result| {
                //     Msg::BeepInserted
                // })
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Msg> {
        w::container(Column::new().push(w::text("WHAT!!")))
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
    let flags = Flags {};

    let mut settings = iced::Settings::with_flags(flags);

    settings.default_font = Font::with_name("Fira Code");

    Model::run(settings).map_err(Error::IcedRunError)
}
