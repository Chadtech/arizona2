use crate::capability::logging::LogCapability;
use crate::domain::logger::Level;
use crate::worker::Worker;

impl LogCapability for Worker {
    fn log(&self, level: Level, message: &str) {
        self.logger.log(level, message);
    }
}
