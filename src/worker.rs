mod memory_capability;
mod person_capability;
mod person_identity_capability;

use crate::{db, nice_display::NiceDisplay, open_ai_key::OpenAiKey};
use sqlx::postgres::PgPoolOptions;
use sqlx::Postgres;
use std::env::VarError;
use std::time::Duration;

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
    PoolAcquire(sqlx::Error),
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
            InitError::PoolAcquire(err) => {
                format!(
                    "Error acquiring a database connection from the pool\n{}",
                    err
                )
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
                .min_connections(2)
                .idle_timeout(Duration::from_secs(600))
                .max_connections(19)
                .test_before_acquire(true)
                .connect(&postgres_conn_url)
                .await
                .map_err(InitError::PoolConnection)?
        };

        sqlx::query("SELECT 1")
            .execute(&sqlx_pool)
            .await
            .map_err(InitError::PoolAcquire)?;

        Ok(Worker {
            open_ai_key,
            reqwest_client: reqwest::Client::new(),
            sqlx: sqlx_pool,
        })
    }

    pub async fn warm_up_db_connection(&self) -> Result<(), String> {
        sqlx::query("SELECT 1")
            .execute(&self.sqlx)
            .await
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}
