pub struct PersonName(String);

impl PersonName {
    pub fn from_string(name: String) -> Self {
        Self(name)
    }

    pub fn to_string(&self) -> &str {
        &self.0
    }
}
