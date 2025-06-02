use crate::db;
use crate::nice_display::NiceDisplay;
use chrono::NaiveDateTime;
use std::{fs, io};
use tokio_postgres::NoTls;

const SEPARATOR: &str = "____";
const DATE_FORMAT: &str = "%Y-%m-%d-%H:%M:%S";

struct Migration {
    name: String,
    timestamp: i64,
}

pub enum NewMigrationError {
    WritingFile(io::Error),
    ConnectingToDb(tokio_postgres::Error),
}

pub enum RunError {
    GetMigrations(GetMigrationsError),
    DbConfig(db::ConfigError),
    ReadingMigrationFile(io::Error),
    ExecutingMigration(tokio_postgres::Error),
    ConnectingToDb(tokio_postgres::Error),
}

pub enum GetMigrationsError {
    GettingMigrations(io::Error),
    ParsingDateFromFileName {
        file_name: String,
        err: chrono::ParseError,
    },
    SplittingFileName {
        file_name: String,
    },
}

impl NiceDisplay for NewMigrationError {
    fn message(&self) -> String {
        match self {
            NewMigrationError::WritingFile(err) => format!("Error writing migration file: {}", err),
            NewMigrationError::ConnectingToDb(err) => {
                format!("Error connecting to database: {}", err)
            }
        }
    }
}
impl NiceDisplay for RunError {
    fn message(&self) -> String {
        match self {
            RunError::GetMigrations(err) => err.message(),
            RunError::DbConfig(err) => err.message(),
            RunError::ReadingMigrationFile(err) => format!("Error reading migration file: {}", err),
            RunError::ExecutingMigration(err) => format!("Error executing migration: {}", err),
            RunError::ConnectingToDb(err) => {
                format!("Error connecting to database: {}", err)
            }
        }
    }
}

impl NiceDisplay for GetMigrationsError {
    fn message(&self) -> String {
        match self {
            GetMigrationsError::GettingMigrations(err) => {
                format!("Error getting migrations: {}", err)
            }
            GetMigrationsError::ParsingDateFromFileName { file_name, err } => {
                format!("Error parsing date from file name '{}': {}", file_name, err)
            }
            GetMigrationsError::SplittingFileName { file_name } => {
                format!("Error splitting file name '{}'", file_name)
            }
        }
    }
}

pub async fn new(name: String) -> Result<(), NewMigrationError> {
    let now = chrono::Utc::now().format(DATE_FORMAT).to_string();

    let new_migration_file_name = format!("{}{}{}.sql", now, SEPARATOR, name);

    fs::write(
        format!("./db/migrations/{}", new_migration_file_name),
        r#"-- ${name}

BEGIN;
-- Write your migration here
COMMIT;"#
            .replace("${name}", name.as_str()),
    )
    .map_err(NewMigrationError::WritingFile)?;

    Ok(())
}

pub async fn run() -> Result<(), RunError> {
    // Get migrations
    let migrations: Vec<Migration> = get_migrations().map_err(RunError::GetMigrations)?;
    let migrations_len = migrations.len();

    // Connect to the database
    let config = db::Config::load().await.map_err(RunError::DbConfig)?;

    println!(
        "Should I run migrations at host {} with password {}? (Y/n): ",
        config.host, config.password
    );

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    // Trim whitespace and convert to uppercase
    let input = input.trim().to_uppercase();

    if input != "Y" {
        println!("Okay, I won't run the migrations");
        return Ok(());
    }

    let (client, connection) = {
        let connect_string = format!(
            "host={} user={} password={} dbname=audio_storer",
            config.host, config.user, config.password
        );

        tokio_postgres::connect(connect_string.as_str(), NoTls)
            .await
            .map_err(RunError::ConnectingToDb)?
    };

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    // Useful for the print statements below
    let mut ran_at_least_one_migration = false;
    for (index, migration) in migrations.into_iter().enumerate() {
        let human_migration_name = {
            let without_number = migration
                .name
                .split(SEPARATOR)
                .collect::<Vec<&str>>()
                .last()
                .unwrap()
                .to_string();

            let name_len = without_number.len();

            without_number[..name_len - 4].to_string()
        };

        println!(
            "Running {}/{}, {}",
            index + 1,
            migrations_len,
            human_migration_name
        );

        let migration_file_path = format!("./db/migrations/{}", migration.name);

        let migration_file_content =
            fs::read_to_string(migration_file_path).map_err(RunError::ReadingMigrationFile)?;

        client
            .batch_execute(migration_file_content.as_str())
            .await
            .map_err(RunError::ExecutingMigration)?;

        ran_at_least_one_migration = true;
    }

    let finish_msg = if ran_at_least_one_migration {
        "Done!"
    } else {
        "You are already up to date, no migrations run!"
    };

    println!("{}", finish_msg);

    Ok(())
}

fn get_migrations() -> Result<Vec<Migration>, GetMigrationsError> {
    let migration_dir_content =
        fs::read_dir("./db/migrations").map_err(GetMigrationsError::GettingMigrations)?;

    let mut migrations = migration_dir_content
        .filter_map(|file| {
            let file = file.unwrap();
            file.path().extension().and_then(|ext| {
                let file_type = file.file_type().unwrap();

                if ext.to_str().unwrap() == "sql" && file_type.is_file() {
                    let file_name = file.file_name();
                    let file_name_str = file_name.to_str().unwrap();

                    Some(file_name_str.to_string())
                } else {
                    None
                }
            })
        })
        .map(
            |file_name: String| match file_name.split(SEPARATOR).collect::<Vec<&str>>().first() {
                Some(n) => NaiveDateTime::parse_from_str(n, DATE_FORMAT)
                    .map_err(|err| GetMigrationsError::ParsingDateFromFileName {
                        err,
                        file_name: file_name.clone(),
                    })
                    .map(|dt| Migration {
                        name: file_name,
                        timestamp: dt.and_utc().timestamp(),
                    }),
                None => Err(GetMigrationsError::SplittingFileName { file_name })?,
            },
        )
        .collect::<Result<Vec<Migration>, GetMigrationsError>>()?;

    migrations.sort_by(|m0, m1| m0.timestamp.cmp(&m1.timestamp));

    Ok(migrations)
}
