#[derive(Debug, Clone)]
pub struct PersonName(String);

impl PersonName {
    pub fn from_string(name: String) -> Self {
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl From<String> for PersonName {
    fn from(value: String) -> Self {
        PersonName(value)
    }
}
