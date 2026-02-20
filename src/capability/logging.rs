use crate::domain::logger::Level;

pub trait LogCapability {
    fn log(&self, level: Level, message: &str);
}
