# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Arizona2 is a Rust application for simulating autonomous AI people who can think, remember, feel, and take action on their own. The goal is to create AI agents whose memories, emotional states, and actions are processed through LLMs, allowing them to live independent lives.

**Key Vision**: AI people should be able to take initiative - deciding on their own to message other AI people, interact in scenes, or reach out to anyone based on their thoughts and feelings. The major milestone is true autonomy where they act based on their own volition rather than just responding to external triggers.

### Domain Model as Simulation

- **Person**: An AI agent with their own identity, personality, and agency
- **Memory**: Experiences and knowledge that shape how an AI person thinks and behaves
- **StateOfMind**: The emotional/mental state influencing an AI person's decisions and actions
- **Scene**: Virtual spaces where AI people exist and can interact with each other
- **Job**: Asynchronous tasks processed through LLMs for AI thoughts, decisions, and actions

The system uses PostgreSQL for persistence and includes an Iced-based GUI admin interface for observing and managing the AI people.

## Development Commands

### Build and Run

```bash
# Build the project
cargo build

# Run the web server (port 1754)
cargo run -- run

# Run the admin UI (GUI application)
cargo run -- admin-ui

# Run the job runner (background worker)
cargo run -- run-job-runner
```

### Database Management

```bash
# Create a new migration
cargo run -- new-migration <migration_name>

# Run all pending migrations
cargo run -- run-migrations
```

### Testing

```bash
# Run all tests
cargo test

# Run tests for a specific module
cargo test <module_name>

# Run a specific test
cargo test <test_name>
```

## Architecture

### Core Components

**Worker** (`src/worker.rs`): Central component that provides database connection pooling (sqlx), OpenAI API access, and
HTTP client. All capabilities are implemented as traits on Worker.

**Capabilities** (`src/capability/`): Trait-based abstraction layer for domain operations. Each domain entity (Job,
Person, Memory, etc.) has a corresponding capability trait defining its operations. This enables dependency injection
and easier testing with mock implementations.

**Domain** (`src/domain/`): Core domain types including:

- Strongly-typed UUIDs for each entity (JobUuid, PersonUuid, etc.)
- Domain models (Job, Person, Memory, Scene, etc.)
- Business logic and validation

**Job Runner** (`src/job_runner.rs`): Background worker that continuously polls for jobs from the database and processes
them. Jobs are processed sequentially using a queue pattern (pop_next_job → process → mark_job_finished).

**Admin UI** (`src/admin_ui/`): Iced-based desktop GUI for managing persons, memories, scenes, and jobs. State is
persisted to `storage.json` in the project root.

### Database

- Database: PostgreSQL (database name: `arizona2`)
- Migration system: Custom timestamp-based migrations in `db/migrations/`
- Connection: Both tokio-postgres (for migrations) and sqlx (for application)
- Migrations use format: `YYYY-MM-DD-HH:MM:SS____description.sql`

Environment variables (`.env`):

- `DATABASE_USER`: Database username
- `DATABASE_PASSWORD`: Database password
- `DATABASE_HOST`: Database host
- `OPEN_AI_API_KEY`: OpenAI API key for AI features

### Error Handling

The codebase uses a custom `NiceDisplay` trait (`src/nice_display.rs`) for user-friendly error messages. All error types
implement this trait to provide contextual error information.

### Key Patterns

1. **Capability Pattern**: Operations are defined as traits (e.g., `JobCapability`, `PersonCapability`) and implemented
   on `Worker`. This allows for easy mocking in tests.

2. **UUID Type Safety**: Each domain entity has a strongly-typed UUID wrapper (e.g., `JobUuid`, `PersonUuid`) to prevent
   mixing IDs across entities.

3. **Job Queue**: Background jobs are managed through a simple queue pattern where jobs are stored in the database and
   processed by the job runner.

## Key Files

- `src/main.rs`: CLI entry point with command routing
- `src/worker.rs`: Core Worker struct with database and API connections
- `src/job_runner.rs`: Background job processing loop
- `src/migrations.rs`: Custom database migration system
- `src/admin_ui.rs`: Desktop GUI application entry point
- `src/capability/`: Trait definitions for domain operations
- `src/domain/`: Domain models and business logic
- `db/migrations/`: SQL migration files
