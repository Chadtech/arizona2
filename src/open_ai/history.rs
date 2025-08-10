use crate::open_ai::message::Message;
use crate::open_ai::role::Role;

pub struct History {
    messages: Vec<Message>,
}

impl History {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, role: Role, content: &str) {
        self.messages.push(Message::new(role, content));
    }

    pub fn get_messages(&self) -> &[Message] {
        &self.messages
    }
}
