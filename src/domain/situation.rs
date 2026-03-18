use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct Situation {
    person_name: String,
    scene_name: Option<String>,
    scene_description: Option<String>,
    participants: Vec<String>,
    messages: Vec<String>,
}

pub struct Input {
    pub person_name: String,
    pub particpants: Vec<String>,
    pub messages: Vec<String>,
}

impl Situation {
    pub fn new(input: Input) -> Self {
        Self {
            person_name: input.person_name,
            scene_name: None,
            scene_description: None,
            participants: input.particpants,
            messages: input.messages,
        }
    }

    pub fn to_people_present_text(&self) -> String {
        let participant_list = if self.participants.is_empty() {
            "none".to_string()
        } else {
            self.participants.join(", ")
        };

        let scene_text = match &self.scene_name {
            Some(name) => match &self.scene_description {
                None => {
                    format!("{} is in the scene \"{}\".", self.person_name, name)
                }
                Some(scene_description) => {
                    format!(
                        "{} is in the scene \"{}\". {}",
                        self.person_name, name, scene_description
                    )
                }
            },
            None => "".to_string(),
        };

        format!(
            "{}\n\nPeople present (complete list): {}",
            scene_text, participant_list
        )
    }
}

impl Display for Situation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let messages_block = if self.messages.is_empty() {
            "no new messages".to_string()
        } else {
            self.messages.join("\n")
        };

        let s = format!(
            "{}\n\nNew messages received (oldest to newest):\n{}",
            self.to_people_present_text(),
            messages_block
        );

        write!(f, "{}", s)
    }
}
