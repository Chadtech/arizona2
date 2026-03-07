use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct PersonName(String);

impl PersonName {
    pub fn from_string(name: String) -> Self {
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for PersonName {
    fn from(value: String) -> Self {
        PersonName(value)
    }
}

impl Display for PersonName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
