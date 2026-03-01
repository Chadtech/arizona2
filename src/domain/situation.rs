use crate::domain::event::Event;

#[derive(Clone, Debug)]
pub struct Situation {
    person_name: String,
    scene_name: String,
    scene_description: String,
    particpants: Vec<String>,
    messages: Vec<String>,
    events: Option<Vec<Event>>,
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
            particpants: input.particpants,
            messages: input.messages,
            events: None,
        }
    }

    pub fn with_events(self, events: Vec<Event>) -> Self {
        let mut ret = self.clone();
        ret.events = Some(events);
        ret
    }

    pub fn to_string(&self) -> String {
        let participant_list = if self.particpants.is_empty() {
            "none".to_string()
        } else {
            self.particpants.join(", ")
        };

        let messages_block = if self.messages.is_empty() {
            "no new messages".to_string()
        } else {
            self.messages.join("\n")
        };

        format!(
            "{} is in the scene \"{}\". {}\n\nPeople present (complete list): {}\n\nNew messages received (oldest to newest):\n{}",
            self.person_name,
            self.scene_name,
            self.scene_description,
            participant_list,
            messages_block
        )
    }
}
