use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct Situation {
    person_name: String,
    scene_name: String,
    scene_description: String,
    participants: Vec<String>,
    messages: Vec<String>,
}

pub struct Input {
    pub person_name: String,
    pub scene_name: String,
    pub scene_description: String,
    pub particpants: Vec<String>,
    pub messages: Vec<String>,
}

impl Situation {
    pub fn new(input: Input) -> Self {
        Self {
            person_name: input.person_name,
            scene_name: input.scene_name,
            scene_description: input.scene_description,
            participants: input.particpants,
            messages: input.messages,
        }
    }
}

impl Display for Situation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let participant_list = if self.participants.is_empty() {
            "none".to_string()
        } else {
            self.participants.join(", ")
        };

        let messages_block = if self.messages.is_empty() {
            "no new messages".to_string()
        } else {
            self.messages.join("\n")
        };

        let s = format!(
            "{} is in the scene \"{}\". {}\n\nPeople present (complete list): {}\n\nNew messages received (oldest to newest):\n{}",
            self.person_name,
            self.scene_name,
            self.scene_description,
            participant_list,
            messages_block
        );

        write!(f, "{}", s)
    }
}
