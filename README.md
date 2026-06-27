# arizona2

Arizona2 is a Rust/PostgreSQL project for simulating AI people in shared
scenes. The current runtime is split between an Iced admin UI and a background
job runner.

## Quick Start

Create a local `.env` with:

- `DATABASE_USER`
- `DATABASE_PASSWORD`
- `DATABASE_HOST`
- `OPEN_AI_API_KEY`

Then run:

```bash
cargo run -- run-migrations
cargo run -- admin-ui
```

In a separate terminal, start the background worker:

```bash
cargo run -- run-job-runner
```

To see every implemented command:

```bash
cargo run -- --help
```

There is currently no web-server command; `cargo run -- run` is not implemented.

## Development

```bash
cargo check
cargo test
```

The PostgreSQL integration tests in `tests/worker_integration.rs` are ignored by
default and require a configured `arizona2_test` database.
