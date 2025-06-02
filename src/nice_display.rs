use std::fmt::Display;

pub struct NiceError {
    content: String,
}

impl Display for NiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.content)
    }
}

pub trait NiceDisplay {
    fn message(&self) -> String;
    fn to_nice_error(&self) -> NiceError {
        NiceError {
            content: self.message(),
        }
    }
}
