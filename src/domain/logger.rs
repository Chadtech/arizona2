use std::path::Path;

#[derive(Clone, Debug)]
pub enum Level {
    Debug,
    Info,
    Warning,
    Error,
}

impl Level {
    pub fn to_priority(&self) -> u8 {
        match self {
            Level::Debug => 0,
            Level::Info => 1,
            Level::Warning => 2,
            Level::Error => 3,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Level::Debug => "DEBUG".to_string(),
            Level::Info => "INFO".to_string(),
            Level::Warning => "WARNING".to_string(),
            Level::Error => "ERROR".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum LogTo {
    File,
    Console,
}

#[derive(Clone, Debug)]
pub struct Logger {
    level: Level,
    log_to: LogTo,
}

impl Logger {
    pub fn log(&self, level: Level, message: &str) {
        if self.should_log(&level) {
            if let Err(err) = self.log_helper(level, message) {
                self.log_helper(Level::Warning, &format!("Failed to log message: {}", err))
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to log warning message: {}", e);
                    })
            }
        }
    }

    fn log_helper(&self, level: Level, message: &str) -> Result<(), String> {
        let log_message = format!("[{}] {}\n", level.to_string(), message);

        match self.log_to {
            LogTo::Console => {
                print!("{}", log_message);
                Ok(())
            }
            LogTo::File => {
                if !Path::new("logs").is_dir() {
                    std::fs::create_dir("logs")
                        .map_err(|e| format!("Failed to create logs directory: {}", e))?;
                }

                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("logs/log.txt")
                    .and_then(|mut file| {
                        std::io::Write::write_all(&mut file, log_message.as_bytes())
                    })
                    .map_err(|e| format!("Failed to write to log file: {}", e))
            }
        }
    }

    fn should_log(&self, level: &Level) -> bool {
        level.to_priority() >= self.level.to_priority()
    }

    pub fn init(level: Level) -> Self {
        Self {
            level,
            log_to: LogTo::Console,
        }
    }

    pub fn log_to_file(mut self) -> Self {
        self.log_to = LogTo::File;
        self
    }
}
