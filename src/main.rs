mod admin_ui;
mod capability;
mod db;
mod domain;
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
}

enum Error {
    ActixWeb(WebServerError),
    NewMigration(migrations::NewMigrationError),
    RunMigrations(migrations::RunError),
    EnvVars(dotenv::Error),
    AdminUi(admin_ui::Error),
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
        }
    }
}

#[actix_web::main]
async fn main() -> Result<(), String> {
    nice_main()
        .await
        .map_err(|err| err.to_nice_error().to_string())
}

async fn nice_main() -> Result<(), Error> {
    dotenv::dotenv().map_err(Error::EnvVars)?;

    let cmd = Cmd::parse();

    match cmd {
        Cmd::Run => run_server().await.map_err(Error::ActixWeb),
        Cmd::NewMigration { migration_name } => migrations::new(migration_name)
            .await
            .map_err(Error::NewMigration),
        Cmd::RunMigrations => migrations::run().await.map_err(Error::RunMigrations),
        Cmd::AdminUi => admin_ui::run().await.map_err(Error::AdminUi),
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
