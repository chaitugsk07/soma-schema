+++
title = "Consuming in your project"
description = "How to adopt soma-schema in another repo — the dependency forms and the non-negotiable invariants."
weight = 70
+++

This page covers the contract for adopting soma-schema in a soma service (soma-iam, soma-vault, or any other consumer). It is condensed from the `CONSUMING.md` file checked into the soma-schema repo.

## Cargo dependency

```toml
# Active development (sibling clone) — library only, no CLI overhead:
soma-schema = { path = "../soma-schema", default-features = false }

# Pinned to a git tag:
soma-schema = { git = "https://github.com/chaitugsk07/soma-schema", tag = "v0.3.0", default-features = false }

# From crates.io:
soma-schema = { version = "0.3", default-features = false }
```

Use `default-features = false` when embedding the library. The default `cli` feature pulls in `clap`, which you do not need in a service binary.

## Library integration at startup

```rust
use soma_schema::{Migrator, PostgresConfig, PostgresDriver};

// Pool must allow >= 2 connections.
let driver = PostgresDriver::new(pool.clone(), PostgresConfig {
    schema:            Some("soma_iam".into()),
    advisory_lock_key: 0x_50A_1A33, // unique per service
    ..Default::default()
})?;

Migrator::from_root("migrations").up(&driver).await?;
```

`PostgresConfig` defaults: `table = "00_schema_migrations"`, `advisory_lock_key = 918273645`. Always override `schema` and always override `advisory_lock_key` when services share one Postgres database.

## CLI in a deploy pipeline

```sh
soma-schema --database-url "$DATABASE_URL" --schema soma_iam --migrations migrations up
soma-schema --database-url "$DATABASE_URL" --schema soma_iam --migrations migrations status
soma-schema --database-url "$DATABASE_URL" --schema soma_iam --migrations migrations down --steps 1
```

## Non-negotiable invariants

These are the rules that, if broken, cause immediate errors or silent database corruption.

**1. Never edit an applied migration file.**
The checksum is SHA-256 of the whole file — UP and DOWN together. Any edit, including a comment change, triggers `ChecksumDrift` on the next command. To change deployed schema: write a new migration.

**2. Every new `.sql` file must be in `migration-order.yaml`.**
Add it to the correct version block in apply order. A file not in the manifest → `OrphanMigration` error. A manifest entry with no file → `MissingFile` error.

**3. Write a DOWN section for every migration unless it is genuinely irreversible.**
DOWN must undo UP in FK-safe reverse order — drop dependent objects before parents. Because rollback order is the reverse of manifest position, manifest order must be FK-correct going forward.

**4. Seeds are idempotent.**
UP SQL uses `ON CONFLICT DO NOTHING` (or equivalent). This makes re-running after a rollback safe.

**5. One schema per service.**
Set `PostgresConfig.schema` (e.g. `soma_iam`). The `00_schema_migrations` tracking table lives in that schema. The `00_setup/01_schema.sql` file must `CREATE SCHEMA IF NOT EXISTS` the schema before anything else.

**6. Pool `max_connections >= 2`.**
One connection is held for the run-scoped advisory lock. `PostgresDriver::new` returns `PoolTooSmall` if the pool cannot provide two connections.

**7. Unique `advisory_lock_key` per service when services share one database.**
Postgres advisory locks are database-global by key. Two services using the same key will serialize against each other even if they are in separate schemas. Pick a distinct `i64` constant per service and leave a comment identifying the service.

**8. `00_setup/` SQL must be idempotent.**
Use `CREATE SCHEMA IF NOT EXISTS`, `CREATE OR REPLACE FUNCTION`, and similar. This directory runs on every `up()` and is never tracked.

## Adding a migration — the workflow

1. Pick or create the correct version folder under `01_migrated/` (or `02_inprogress/` for in-flight work).
2. Create `<YYYYMMDD>_<NN>_<name>.sql` with a UP section, the `-- DOWN ==` delimiter, and a DOWN section.
3. Add the entry to `migration-order.yaml` in the right version block with `created`, `author`, and `why` fields.
4. Run `status` to confirm the new file appears as pending.
5. Run `up` (or let the service start) to apply it.
6. Never touch the file again once it has been applied to any environment.

## CLAUDE.md block for the consumer repo

Paste this into the consumer repo's own `CLAUDE.md` so Claude Code knows the migration rules when working in that repo:

```
## Database migrations — soma-schema

All database migrations use soma-schema (https://github.com/chaitugsk07/soma-schema).
The canonical integration guide is soma-schema/CONSUMING.md. When wiring migrations or
writing SQL, invoke /soma-schema (runner rules) and /db-standards (SQL content rules).

Non-negotiable:
1. Never edit an applied migration file. Checksum drift = immediate error on next run.
2. Every new .sql must be listed in migration-order.yaml (correct version, apply order).
3. Write a DOWN section for every migration unless it is irreversible. Manifest order
   must be FK-correct forward; rollback order is the exact reverse.
4. Seeds use ON CONFLICT DO NOTHING in UP so re-runs are safe.
5. One schema per service. 00_setup/ must CREATE SCHEMA IF NOT EXISTS it.
6. Pool max_connections >= 2 (one held for advisory lock).
7. Unique advisory_lock_key per service when services share one Postgres database.
8. 00_setup/ SQL is idempotent (IF NOT EXISTS, CREATE OR REPLACE) — runs every up().
9. SQL content follows /db-standards (plain-Postgres baseline, pgcrypto only).
```
