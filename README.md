# soma-schema

[![CI](https://github.com/chaitugsk07/soma-schema/actions/workflows/ci.yml/badge.svg)](https://github.com/chaitugsk07/soma-schema/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/soma-schema?style=flat-square)](https://crates.io/crates/soma-schema)
[![docs.rs](https://img.shields.io/docsrs/soma-schema?style=flat-square)](https://docs.rs/soma-schema)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue?style=flat-square)](LICENSE)
[![GitHub](https://img.shields.io/badge/github-chaitugsk07%2Fsoma--schema-8da0cb?style=flat-square&logo=github)](https://github.com/chaitugsk07/soma-schema)

Plain SQL database migrations for Rust — with full-file drift detection, manifest-defined ordering, and run-scoped advisory locking.

---

soma-schema is a standalone Postgres migration tool that ships as both a library crate and a CLI binary. You write plain SQL files with UP and DOWN sections, maintain a `migration-order.yaml` manifest that defines apply and rollback order explicitly, and soma-schema handles the rest: advisory locking so concurrent runners can't collide, SHA-256 checksums over the entire file so any post-deploy edit is caught, and atomic apply+track so a crash can't leave a half-applied state. It has no ORM dependency and no opinions about your application framework.

---

## Why soma-schema

Most Rust migration tools order migrations by filename sort and hash only the UP SQL. That works until it doesn't: a filename collision, a rollback that re-creates a table FK dependencies expect to exist, or someone editing a deployed DOWN section and not noticing the drift. soma-schema addresses each of these directly.

- **Manifest-defined order.** `migration-order.yaml` lists every migration explicitly. Apply order is that list top-to-bottom; rollback order is the exact reverse of manifest position — not filename sort. Deterministic FK-safe rollback without naming conventions. (sqlx-migrate, refinery, and diesel_migrations all order by filename.)

- **Full-file checksum drift detection.** The SHA-256 checksum covers the entire file — UP and DOWN together. Editing the DOWN section of a deployed migration is caught as `ChecksumDrift` the next time any command runs. (sqlx hashes UP-only; refinery hashes metadata differently; diesel has no drift detection.)

- **Apply and track in one transaction.** The migration SQL and its tracking-table row commit atomically. A crash between those two operations is not possible; you will never see a migration that ran but has no record, or a record for a migration that never ran.

- **Run-scoped advisory lock.** A single Postgres advisory lock is acquired once at the start of `up`, `down`, or `status` and held via a RAII guard until the call returns — even on panic. Concurrent runners block rather than collide.

---

## Install

### As a library

```toml
[dependencies]
soma-schema = "0.2"
```

Or:

```sh
cargo add soma-schema
```

### As a CLI

```sh
cargo install soma-schema
```

The binary requires the `cli` feature, which is on by default.

---

## AI-native: your agent writes the migrations

Traditional migration tools (Flyway, Liquibase, sqlx-migrate) were built for humans typing commands. soma-schema ships the agent contract so your AI writes correct migrations from the start.

Paste this block into your repo's `AGENTS.md` or `CLAUDE.md` (works with any agent — Claude, Cursor, Copilot, etc.):

```text
## Database migrations — soma-schema

All database migrations use soma-schema (https://github.com/chaitugsk07/soma-schema).

Non-negotiable:
1. Never edit an applied migration file. Checksum drift = immediate error on next run.
2. Every new .sql must be listed in migration-order.yaml (correct version, apply order).
3. Write a DOWN section for every migration unless it is genuinely irreversible. Manifest
   order must be FK-correct forward; rollback order is the exact reverse.
4. Seeds use ON CONFLICT DO NOTHING in UP so re-runs are safe.
5. One schema per service. 00_setup/ must CREATE SCHEMA IF NOT EXISTS it.
6. Pool max_connections >= 2 (one held for advisory lock).
7. Unique advisory_lock_key per service when services share one Postgres database.
8. 00_setup/ SQL is idempotent (IF NOT EXISTS, CREATE OR REPLACE) — runs every up().

Adding a migration:
- Create <YYYYMMDD>_<NN>_<name>.sql with UP + "-- DOWN ==" + DOWN.
- Add it to migration-order.yaml (created/author/why).
- Run status to confirm pending, then up to apply.
- Never touch the file again once applied to any environment.
```

With that block in place, an agent generating a migration will follow the invariants automatically — correct file format, manifest entry, idempotent seeds, and a proper DOWN section.

---

## 60-second quickstart

```sh
# Scaffold a migrations directory.
soma-schema init migrations/

# Edit migrations/00_setup/01_schema.sql — add your CREATE SCHEMA statement.
# Add your first migration to migrations/01_migrated/1/ and list it in migration-order.yaml.

# Apply everything pending.
soma-schema --database-url "$DATABASE_URL" --migrations migrations/ up

# Check what's applied and what's pending.
soma-schema --database-url "$DATABASE_URL" --migrations migrations/ status

# Roll back the last migration.
soma-schema --database-url "$DATABASE_URL" --migrations migrations/ down

# Roll back the last 3 migrations.
soma-schema --database-url "$DATABASE_URL" --migrations migrations/ down --steps 3
```

`DATABASE_URL` can also be set as an environment variable; `--database-url` overrides it.

### CLI flags

| Flag | Env var | Default | Description |
| --- | --- | --- | --- |
| `--database-url` | `DATABASE_URL` | — | Postgres connection URL |
| `--migrations` | — | `migrations` | Path to migrations root |
| `--schema` | — | (connection default) | Target schema |
| `--table` | — | `00_schema_migrations` | Tracking table name |

---

## Visual explorer

soma-schema can build a self-contained HTML explorer from your migrations directory — no database connection needed.

```sh
soma-schema --migrations migrations/ explorer
```

This writes an HTML file and opens it in your browser. The page shows a schema ERD, a version-grouped migration timeline, and seed-data tables.

| Flag | Default | Description |
| --- | --- | --- |
| `--format html\|json` | `html` | HTML page or raw JSON |
| `--out <path>` | temp file (html) / stdout (json) | Write to a specific path |
| `--no-open` | off | Skip opening the browser |

---

## Migration file format

Each `.sql` file holds both directions separated by a delimiter line:

```sql
-- UP section (everything before the separator)
CREATE TABLE myschema.widgets (id UUID PRIMARY KEY DEFAULT gen_random_uuid());

-- DOWN ==
DROP TABLE IF EXISTS myschema.widgets;
```

The separator is exactly `-- DOWN ==` (trimmed). The DOWN section is optional — soma-schema will error if you attempt to roll back a migration that lacks one.

Seeds are ordinary migration files whose UP SQL is idempotent (`ON CONFLICT DO NOTHING`).

---

## Migration layout

```text
migrations/
  migration-order.yaml          # authoritative ordered manifest
  00_setup/                     # idempotent bootstrap (schema, extensions, grants)
    01_schema.sql               #   runs before every up(); untracked
  01_migrated/                  # deployed, treat as immutable
    1/                          # version 1 (integer folder name, sorted numerically)
      20260101_01_init.sql
      20260101_02_seed.sql
  02_inprogress/                # staging area for in-flight work (optional)
    2/
      20260201_01_add_widgets.sql
```

### migration-order.yaml

This file is the single source of truth for what migrations exist and in what order:

```yaml
manifest_version: 1
versions:
  - version: 1
    description: "Initial schema"
    migrations:
      - file: "20260101_01_init.sql"
        created: "2026-01-01"
        author: "you"
        why: "Create core tables"
      - file: "20260101_02_seed.sql"
        created: "2026-01-01"
        why: "Seed reference data"
```

Rules:

- Every `.sql` file under a version folder must be listed here. A file on disk but missing from the manifest → `OrphanMigration` error.
- Every manifest entry must resolve to a real file → `MissingFile` error.
- `file` is a bare filename (no path separators, must end in `.sql`).
- `00_setup` files are not listed here and are never orphan-flagged.
- Versions are sorted numerically (`10` comes after `2`).

---

## Tracking table

soma-schema creates a table (default `00_schema_migrations`) in the configured schema:

| Column | Type | Description |
| --- | --- | --- |
| `version` | INTEGER | Version-folder number |
| `file` | VARCHAR(255) | Migration filename |
| `name` | VARCHAR(255) | Filename without `.sql` |
| `checksum` | TEXT | SHA-256 of the full file |
| `description` | TEXT | `why` from migration-order.yaml |
| `batch` | INTEGER | Increments once per `up()` run that applies at least one migration |
| `applied_at` | TIMESTAMPTZ | When it was deployed |
| `applied_by` | TEXT | DB role that deployed it |
| `execution_ms` | INTEGER | How long the migration took |

`revert` deletes the row; the migration becomes pending again. `status` shows applied rows plus pending migrations from the manifest.

---

## Library usage

```rust
use soma_schema::{Migrator, PostgresConfig, PostgresDriver};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = PgPoolOptions::new()
        .max_connections(2) // soma-schema needs >= 2 (one holds the advisory lock)
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

Key types re-exported at the crate root:

| Type | Description |
| --- | --- |
| `Migrator` | Owns the migrations root; drives `up`, `down`, `status`, `scaffold` |
| `PostgresDriver` | Postgres implementation of `MigrationDriver` |
| `PostgresConfig` | Driver config: `schema`, `table`, `advisory_lock_key` |
| `MigrationDriver` | Trait to implement for additional database backends |
| `MigrationStatus` | Return type of `status()`: applied + pending lists |
| `AppliedMigration` | A row from the tracking table |
| `PendingMigration` | A manifest entry not yet applied |
| `Error`, `Result` | Crate error type and alias |

---

## How it compares

| Tool | Lang | Format | Ordering | Checksum | Locking | Lib+CLI | License |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **soma-schema** | Rust | Plain SQL | YAML manifest | Full-file (UP+DOWN) | Advisory, full-run | Both | Apache-2.0 |
| sqlx migrate | Rust | Plain SQL | Filename lexical | UP-only | Advisory (Pg) | Both | MIT/Apache |
| refinery | Rust | Plain SQL | Filename lexical | Metadata hash | None | Lib only | MIT |
| diesel_migrations | Rust | Plain SQL | Filename sort | None | None | Lib (ORM-tied) | MIT/Apache |
| dbmate | Go | Plain SQL | Filename sort | None | Advisory | CLI only | MIT |
| Flyway | JVM | Plain SQL | Version prefix | Full-file | Advisory | Both | Apache + commercial |
| Atlas | Go | HCL/SQL DSL | State-based diff | Schema hash | Advisory | Both | Apache/BSL |

Full analysis with per-tool comparison and "when NOT to choose soma-schema" guidance: [`docs/competitor-analysis.md`](docs/competitor-analysis.md).

---

## Multi-DB design

The `MigrationDriver` trait in `src/driver.rs` is object-safe and database-agnostic. Six async operations — acquire lock, run setup, ensure tracking table, list applied, apply, revert — define the contract. `PostgresDriver` is the only bundled implementation today. Implementing a new backend means implementing `MigrationDriver`; no other code changes.

The manifest ordering, full-file checksum detection, run-scoped locking, and atomic apply+track all live above the driver and are reused by every backend.

**Today:** PostgreSQL only.

**Next:** SQLite — the same plain-SQL UP/DOWN format, `BEGIN IMMEDIATE` as the lock primitive. A natural fit for the SQLite-now → Postgres-later pattern.

**Community-contributed backends** are welcome via the `MigrationDriver` trait. MySQL/MariaDB are the most natural candidates. Open an issue to coordinate.

Full plan, cross-cutting features, and driver contribution guide: [ROADMAP.md](ROADMAP.md).

---

## Tests

Unit tests run without a database:

```sh
cargo test
```

Integration tests require a real Postgres instance:

```sh
TEST_DATABASE_URL="postgresql://user:pass@host:5432/db" cargo test --test integration
```

Every integration test generates a unique throwaway schema (`_sdm_test_<uuid>`), runs inside it, and drops it on teardown — even on panic. Tests never touch `public` or any pre-existing schema.

---

## Contributing

Issues and pull requests are welcome at [github.com/chaitugsk07/soma-schema](https://github.com/chaitugsk07/soma-schema).

## License

Licensed under Apache-2.0. See [LICENSE](LICENSE).
