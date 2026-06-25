+++
title = "Library usage"
description = "Embedding soma-schema as a crate — pool setup, PostgresConfig, and the Migrator API."
weight = 50
+++

The most common deployment pattern for soma services is to run migrations at startup from within the application binary. This page covers the library API for that case.

## Minimal example

```rust
use soma_schema::{Migrator, PostgresConfig, PostgresDriver};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = PgPoolOptions::new()
        // soma-schema needs at least 2 connections:
        // one is held for the run-scoped advisory lock.
        .max_connections(2)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    let driver = PostgresDriver::new(pool, PostgresConfig {
        schema: Some("myapp".into()),
        ..Default::default()
    })?;

    let migrator = Migrator::from_root("migrations/");
    migrator.up(&driver).await?;
    Ok(())
}
```

`PostgresDriver::new` returns `Err(PoolTooSmall)` if the pool's configured `max_connections` is less than 2. Check this at startup, not at migration time.

## PostgresConfig

```rust
let driver = PostgresDriver::new(pool, PostgresConfig {
    schema:            Some("myapp".into()),
    table:             "00_schema_migrations".to_owned(), // default
    advisory_lock_key: 0x_6D79_6170_7020, // unique i64 per service
    ..Default::default()
})?;
```

| Field | Default | Description |
|---|---|---|
| `schema` | connection default | Schema for the tracking table and your migrations |
| `table` | `"00_schema_migrations"` | Tracking table name |
| `advisory_lock_key` | `918273645` | Postgres advisory lock key (database-global) |

Always set `schema` in a real service. Always set a unique `advisory_lock_key` if multiple services share one Postgres database — advisory locks are keyed globally within a database session. Two services using the default key would serialize against each other.

## Migrator methods

`Migrator::from_root(root)` accepts any value implementing `Into<PathBuf>`. It does not read the filesystem at construction time — that happens when you call `up`, `down`, or `status`.

```rust
let migrator = Migrator::from_root("migrations/");

// Apply all pending migrations.
migrator.up(&driver).await?;

// Roll back the most recently applied migration.
migrator.down(&driver, 1).await?;

// Roll back the last N migrations.
migrator.down(&driver, 3).await?;

// Return applied and pending migration lists.
let status = migrator.status(&driver).await?;
println!("{} applied, {} pending",
    status.applied.len(),
    status.pending.len());
```

## Key types

| Type | Description |
|---|---|
| `Migrator` | Owns the migrations root; drives `up`, `down`, `status`, `scaffold` |
| `PostgresDriver` | Postgres implementation of `MigrationDriver` |
| `PostgresConfig` | Driver config: `schema`, `table`, `advisory_lock_key` |
| `MigrationDriver` | Trait to implement for additional database backends |
| `MigrationStatus` | Return type of `status()`: applied + pending lists |
| `AppliedMigration` | A row from the tracking table |
| `PendingMigration` | A manifest entry not yet applied |
| `Error`, `Result` | Crate error type and alias |

All are re-exported at the crate root.

## Error handling

soma-schema uses a typed `Error` enum. `Error` is `#[non_exhaustive]`, so match arms must include a wildcard.

The variants you are most likely to encounter at runtime:

- `ChecksumDrift { version: u32, file: String }` — the on-disk file no longer matches the checksum recorded when it was applied. The run aborts before any migration executes.
- `OrphanMigration { path: String }` — a `.sql` file exists on disk with no corresponding manifest entry.
- `MissingFile { version: u32, file: String }` — a manifest entry has no corresponding file on disk.
- `PoolTooSmall` — `max_connections < 2`.

```rust
match migrator.up(&driver).await {
    Ok(_) => {}
    Err(soma_schema::Error::ChecksumDrift { version, file }) => {
        eprintln!("drift in v{version}/{file} — restore the original file");
    }
    Err(soma_schema::Error::OrphanMigration { path }) => {
        eprintln!("unlisted file: {path} — add it to migration-order.yaml");
    }
    Err(soma_schema::Error::MissingFile { version, file }) => {
        eprintln!("manifest entry v{version}/{file} has no file on disk");
    }
    Err(e) => return Err(e.into()),
    _ => {}
}
```

Handle `ChecksumDrift` as a deployment error; it should never occur in a correctly managed workflow.

## Pool sizing in production

In a web service, your main pool is often sized 5–20. soma-schema needs only 2 of those connections for the duration of the migration run at startup. You can either share the main pool (as long as `max_connections >= 2`) or create a short-lived dedicated pool just for migrations and close it after `up` returns.

See [Consuming in your project](@/consuming-in-your-project.md) for the full integration contract.
