mod person_identity_capability;

use crate::{db, nice_display::NiceDisplay, open_ai_key::OpenAiKey};
use sqlx::postgres::PgPoolOptions;
use sqlx::Postgres;
use std::env::VarError;

#[derive(Clone, Debug)]
pub struct Worker {
    pub open_ai_key: OpenAiKey,
    pub reqwest_client: reqwest::Client,
    pub sqlx: sqlx::Pool<Postgres>,
}

#[derive(Debug)]
pub enum InitError {
    OpenAiKey(VarError),
    DbConfig(db::ConfigError),
    PoolConnection(sqlx::Error),
}

impl NiceDisplay for InitError {
    fn message(&self) -> String {
        match self {
            InitError::OpenAiKey(err) => format!("OpenAI API key error: {}", err),
            InitError::DbConfig(err) => {
                format!("Database configuration error\n{}", err.message())
            }
            InitError::PoolConnection(err) => {
                format!("Error connecting to the database pool\n{}", err)
            }
        }
    }
}

impl Worker {
    pub async fn new() -> Result<Self, InitError> {
        let open_ai_key = OpenAiKey::from_env().map_err(InitError::OpenAiKey)?;

        let db_info = db::Config::load()
            .await
            .map_err(|err| InitError::DbConfig(err))?;

        let sqlx_pool = {
            let postgres_conn_url = format!(
                "postgres://{}:{}@{}/arizona2",
                db_info.user, db_info.password, db_info.host
            );

            PgPoolOptions::new()
                .max_connections(20)
                .connect(postgres_conn_url.as_str())
                .await
                .map_err(InitError::PoolConnection)?
        };

        Ok(Worker {
            open_ai_key,
            reqwest_client: reqwest::Client::new(),
            sqlx: sqlx_pool,
        })
    }
}
