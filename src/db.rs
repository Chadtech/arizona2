use crate::nice_display::NiceDisplay;

pub struct Config {
    pub user: String,
    pub host: String,
    pub password: String,
}

pub enum ConfigError {
    ReadingPassword(dotenv::Error),
    ReadingHost(dotenv::Error),
    ReadingUser(dotenv::Error),
}

impl NiceDisplay for ConfigError {
    fn message(&self) -> String {
        match self {
            ConfigError::ReadingPassword(err) => format!("Error reading DB_PASSWORD: {}", err),
            ConfigError::ReadingHost(err) => format!("Error reading DB_HOST: {}", err),
            ConfigError::ReadingUser(err) => format!("Error reading DB_USER: {}", err),
        }
    }
}

impl Config {
    pub async fn load() -> Result<Config, ConfigError> {
        let password = dotenv::var("DB_PASSWORD").map_err(ConfigError::ReadingPassword)?;
        let host = dotenv::var("DB_HOST").map_err(ConfigError::ReadingHost)?;
        let user = dotenv::var("DB_USER").map_err(ConfigError::ReadingUser)?;

        Ok(Config {
            user,
            host,
            password,
        })
    }
}
