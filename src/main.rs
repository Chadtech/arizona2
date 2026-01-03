mod admin_ui;
mod capability;
mod db;
mod domain;
mod job_runner;
mod migrations;
mod nice_display;
mod open_ai;
mod open_ai_key;
mod person_actions;
mod worker;

use crate::nice_display::NiceDisplay;
use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use clap::Parser;

const PORT: u16 = 1754;

#[derive(Debug, Parser, Clone)]
#[clap(
    author = "Chad Stearns",
    version = "0.1",
    about = "Commands for Aiaday"
)]
enum Cmd {
    Run,
    NewMigration { migration_name: String },
    RunMigrations,
    AdminUi,
    RunJobRunner,
}

enum Error {
    ActixWeb(WebServerError),
    NewMigration(migrations::NewMigrationError),
    RunMigrations(migrations::RunError),
    EnvVars(dotenv::Error),
    AdminUi(admin_ui::Error),
    JobRunner(job_runner::Error),
}

impl NiceDisplay for Error {
    fn message(&self) -> String {
        match self {
            Error::ActixWeb(err) => err.message(),
            Error::NewMigration(err) => err.message(),
            Error::RunMigrations(err) => err.message(),
            Error::EnvVars(err) => {
                format!("Error loading environment variables: {}", err)
            }
            Error::AdminUi(err) => err.message(),
            Error::JobRunner(err) => err.message(),
        }
    }
}

impl Cmd {
    fn log_file_name(&self) -> &str {
        match self {
            Cmd::Run => "server",
            Cmd::NewMigration { .. } => "migrations",
            Cmd::RunMigrations => "migrations",
            Cmd::AdminUi => "admin-ui",
            Cmd::RunJobRunner => "job-runner",
        }
    }
}

#[actix_web::main]
async fn main() -> Result<(), String> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    // Parse command first to determine log file name
    let cmd = Cmd::parse();
    let log_file_name = format!("arizona2-{}.log", cmd.log_file_name());

    // Create logs directory if it doesn't exist
    std::fs::create_dir_all("logs").ok();

    // File appender - each process writes to its own log file
    let file_appender = tracing_appender::rolling::daily("logs", log_file_name);
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    // File layer - captures everything (debug and above)
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false) // No color codes in file
        .with_target(true)
        .with_line_number(true);

    // Console layer - shows info and above
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false);

    // Environment filter - defaults to "warn" for everything except job_runner
    // Job runner gets "info" level, admin_ui stays at "warn"
    // cosmic_text warnings suppressed (font loading warnings are harmless)
    // Can be overridden with RUST_LOG env var
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new("warn,arizona2::job_runner=info,cosmic_text=error")
    });

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(console_layer)
        .init();

    nice_main(cmd)
        .await
        .map_err(|err| err.to_nice_error().to_string())
}

async fn nice_main(cmd: Cmd) -> Result<(), Error> {
    dotenv::dotenv().map_err(Error::EnvVars)?;

    match cmd {
        Cmd::Run => run_server().await.map_err(Error::ActixWeb),
        Cmd::NewMigration { migration_name } => migrations::new(migration_name)
            .await
            .map_err(Error::NewMigration),
        Cmd::RunMigrations => migrations::run().await.map_err(Error::RunMigrations),
        Cmd::AdminUi => admin_ui::run().await.map_err(Error::AdminUi),
        Cmd::RunJobRunner => job_runner::run().await.map_err(Error::JobRunner),
    }
}

#[get("/")]
async fn html() -> impl Responder {
    HttpResponse::Ok().body("Wah")
}

enum WebServerError {
    Bind(std::io::Error),
    Run(std::io::Error),
}

impl NiceDisplay for WebServerError {
    fn message(&self) -> String {
        match self {
            WebServerError::Run(err) => format!("Error running server: {}", err),
            WebServerError::Bind(err) => {
                format!("Error binding server: {}", err)
            }
        }
    }
}

async fn run_server() -> Result<(), WebServerError> {
    println!("Running server on port {}", PORT);
    let r = HttpServer::new(|| {
        App::new().service(html)
        // .service(html).service(elm_js)
    })
    .bind(("127.0.0.1", PORT))
    .map_err(WebServerError::Bind)?
    .run()
    .await
    .map_err(WebServerError::Run)?;

    Ok(r)
}
