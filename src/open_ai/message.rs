use crate::open_ai::role::Role;

pub struct Message {
    role: Role,
    content: String,
}

impl Message {
    pub fn new(role: Role, content: &str) -> Self {
        Self {
            role,
            content: content.to_string(),
        }
    }

    pub fn role(&self) -> &Role {
        &self.role
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}
