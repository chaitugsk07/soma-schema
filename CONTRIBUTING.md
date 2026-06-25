# Contributing to soma-schema

Thank you for your interest in contributing. This document covers how to build the project, run tests, and submit changes.

## Building

```bash
cargo build
```

The CLI binary (`soma-schema`) is gated behind the `cli` feature, which is on by default:

```bash
cargo build --release   # output: target/release/soma-schema
```

## Unit tests

Unit and doc tests run without a database:

```bash
cargo test
```

## Integration tests

Integration tests require a running Postgres instance. They create throwaway schemas named `_sdm_test_<uuid>` and clean up after themselves — they never touch `public` or any pre-existing schema.

```bash
TEST_DATABASE_URL=postgres://user:pass@localhost/mydb cargo test --test integration

# Run a single test by name
TEST_DATABASE_URL=... cargo test --test integration test_checksum_drift_error
```

## Linting

```bash
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

All CI checks must pass before a PR can be merged.

## Adding a new database backend

The only abstraction point is the `MigrationDriver` trait in `src/driver.rs`. To add a new backend, implement `MigrationDriver` for your driver struct — no other code needs to change. See `src/postgres.rs` for a reference implementation. The ROADMAP (`ROADMAP.md`) lists SQLite and MySQL as planned targets.

## Migration file invariants

Never edit the content of a migration file that has already been applied to any environment. soma-schema stores a SHA-256 checksum of the full file (UP + DOWN together) and will refuse to run if it detects drift. This is a core safety guarantee, not a lint suggestion.

## Pull request process

1. Fork the repository and create a branch from `main`.
2. Make your changes. If you are adding behavior, add or update tests.
3. Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` locally.
4. Open a pull request against `main` with a clear description of what changes and why.
5. A maintainer will review and may request changes before merging.

For significant features or design changes, open an issue first to discuss the approach.
