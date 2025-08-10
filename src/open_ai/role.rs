pub enum Role {
    System,
    User,
    Assistant,
}

impl Role {
    pub fn to_str(&self) -> &str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}
