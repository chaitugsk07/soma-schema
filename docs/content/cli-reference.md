+++
title = "CLI reference"
description = "All soma-schema subcommands and flags."
weight = 60
+++

## Global flags

These flags apply to every subcommand that connects to a database.

| Flag | Env var | Default | Description |
|---|---|---|---|
| `--database-url <URL>` | `DATABASE_URL` | — | Postgres connection URL |
| `--migrations <PATH>` | — | `migrations` | Path to the migrations root directory |
| `--schema <SCHEMA>` | — | (connection default) | Target schema for the tracking table |
| `--table <TABLE>` | — | `00_schema_migrations` | Tracking table name |

`--database-url` overrides `DATABASE_URL` if both are set. The URL format is the standard libpq form: `postgres://user:password@host:5432/dbname`.

## `init`

Scaffold a new migrations directory.

```sh
soma-schema init [DIR]
```

`DIR` defaults to `migrations`. Creates the standard layout:

```text
DIR/
  migration-order.yaml
  00_setup/
    01_schema.sql
  01_migrated/
    1/
```

Safe to run on an existing directory — it will not overwrite files that already exist.

## `up`

Apply all pending migrations.

```sh
soma-schema --database-url <URL> --migrations <PATH> up
```

Workflow:

1. Acquires a run-scoped advisory lock.
2. Runs all files in `00_setup/` (idempotent bootstrap).
3. Reads the tracking table to find applied migrations.
4. Recomputes checksums for applied files; aborts with `ChecksumDrift` if any differ.
5. Applies each pending migration in manifest order and records it in the tracking table — both in the same transaction.
6. Releases the lock.

Migrations that fail mid-run leave the database at the last successfully committed migration. The failed migration is not recorded.

## `down`

Roll back applied migrations.

```sh
soma-schema --database-url <URL> --migrations <PATH> down [--steps N]
```

| Flag | Default | Description |
|---|---|---|
| `--steps <N>` | `1` | Number of migrations to roll back |

Rollback order is the exact reverse of manifest position — not filename sort. This is what makes FK-safe rollback deterministic.

Each migration's DOWN SQL and the deletion of its tracking-table row are executed in a single transaction. If a DOWN fails, the transaction is rolled back and the migration remains recorded as applied.

## `status`

Show applied and pending migrations.

```sh
soma-schema --database-url <URL> --migrations <PATH> status
```

Output lists:

- Applied migrations: filename, version, batch number, applied-at timestamp, applied-by (database role), execution time.
- Pending migrations: filename, version, `why` from the manifest.

Checksums are verified during `status` — a drift error will surface here before you attempt `up` or `down`.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | Error (connection failure, drift, orphan, missing file, etc.) |

## Environment variable summary

| Variable | Used by |
|---|---|
| `DATABASE_URL` | All database-connecting subcommands |
