use crate::nice_display::NiceDisplay;

pub struct Config {
    pub user: String,
    pub host: String,
    pub password: String,
}

#[derive(Debug)]
pub enum ConfigError {
    Password(dotenv::Error),
    Host(dotenv::Error),
    User(dotenv::Error),
}

impl NiceDisplay for ConfigError {
    fn message(&self) -> String {
        match self {
            ConfigError::Password(err) => {
                format!("Error reading DATABASE_PASSWORD: {}", err)
            }
            ConfigError::Host(err) => format!("Error reading DATABASE_HOST: {}", err),
            ConfigError::User(err) => format!("Error reading DATABASE_USER: {}", err),
        }
    }
}

impl Config {
    pub async fn load() -> Result<Config, ConfigError> {
        let password = dotenv::var("DATABASE_PASSWORD").map_err(ConfigError::Password)?;
        let host = dotenv::var("DATABASE_HOST").map_err(ConfigError::Host)?;
        let user = dotenv::var("DATABASE_USER").map_err(ConfigError::User)?;

        Ok(Config {
            user,
            host,
            password,
        })
    }
}
