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
            ConfigError::ReadingPassword(err) => {
                format!("Error reading DATABASE_PASSWORD: {}", err)
            }
            ConfigError::ReadingHost(err) => format!("Error reading DATABASE_HOST: {}", err),
            ConfigError::ReadingUser(err) => format!("Error reading DATABASE_USER: {}", err),
        }
    }
}

impl Config {
    pub async fn load() -> Result<Config, ConfigError> {
        let password = dotenv::var("DATABASE_PASSWORD").map_err(ConfigError::ReadingPassword)?;
        let host = dotenv::var("DATABASE_HOST").map_err(ConfigError::ReadingHost)?;
        let user = dotenv::var("DATABASE_USER").map_err(ConfigError::ReadingUser)?;

        Ok(Config {
            user,
            host,
            password,
        })
    }
}
