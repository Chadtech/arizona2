# Repository Guidelines

## Project Structure & Module Organization
This is a Rust workspace with source in `src/`. Core areas include:
- `src/main.rs` for CLI entry and command routing.
- `src/worker.rs` and `src/capability/` for worker capabilities and trait interfaces.
- `src/domain/` for domain models and type-safe UUIDs.
- `src/job_runner.rs` for background job processing.
- `src/admin_ui/` for the Iced desktop UI.
- `db/migrations/` for timestamped SQL migrations.
- `storage.json` for admin UI state and `logs/` for runtime logs.

## Build, Test, and Development Commands
- `cargo build` builds the project.
- `cargo run -- run` starts the web server (port 1754).
- `cargo run -- admin-ui` launches the desktop UI.
- `cargo run -- run-job-runner` runs the background job processor.
- `cargo run -- new-migration <name>` creates a migration file in `db/migrations/`.
- `cargo run -- run-migrations` applies pending migrations.
- `cargo test` runs all tests; `cargo test <module_or_test>` scopes execution.

## Coding Style & Naming Conventions
- Follow standard Rust style (rustfmt defaults, 4-space indentation).
- Use `snake_case` for modules/functions and `CamelCase` for types/traits.
- Prefer explicit error types that implement `NiceDisplay` (`src/nice_display.rs`).
- Avoid `unwrap`/`expect` and placeholder `unwrap_or_else` defaults; return errors instead.
- Keep modules focused; place domain logic in `src/domain/` and IO in capabilities.

## Testing Guidelines
- Tests live inline (e.g., `src/job_runner.rs`), using Rust’s built-in test framework.
- Name tests descriptively (`test_<behavior>` or `it_<does_something>`).
- Run focused tests during changes: `cargo test <test_name>`.

## Commit & Pull Request Guidelines
- Commit messages are currently free-form (recent history shows short summaries and WIP).
- For clarity, prefer concise, present-tense summaries (e.g., “Add memory search UI”).
- PRs should include a brief description, linked issue (if any), and screenshots for UI
  changes (`src/admin_ui/`).

## Configuration & Security Notes
- Use a local `.env` with `DATABASE_USER`, `DATABASE_PASSWORD`, `DATABASE_HOST`,
  and `OPEN_AI_API_KEY`. Do not commit secrets.
- The database is PostgreSQL (default name: `arizona2`).
