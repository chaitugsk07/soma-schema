+++
title = "Tracking table"
description = "The schema of 00_schema_migrations and what each column records."
weight = 40
+++

soma-schema creates a tracking table (default name `00_schema_migrations`) in the configured schema. This table is the source of truth for which migrations have been applied.

## Schema

| Column | Type | Description |
|---|---|---|
| `version` | INTEGER | Version-folder number (the integer directory name under `01_migrated/`) |
| `file` | VARCHAR(255) | Migration filename as listed in the manifest |
| `name` | VARCHAR(255) | Filename without the `.sql` extension |
| `checksum` | TEXT | SHA-256 of the full file content (UP + DOWN) at time of apply |
| `description` | TEXT | The `why` field from `migration-order.yaml` |
| `batch` | INTEGER | Increments once per `up()` run that applies at least one migration |
| `applied_at` | TIMESTAMPTZ | When this migration was applied |
| `applied_by` | TEXT | The database role (`current_user`) that applied it |
| `execution_ms` | INTEGER | Wall-clock time the migration SQL took to run, in milliseconds |

## The `00_` prefix

The `00_` prefix on the default table name causes it to sort before application tables in most database GUI tools. It appears at the top of the table list rather than somewhere alphabetically in the middle. The name is configurable via `PostgresConfig.table` or the `--table` CLI flag.

## Table creation

The table is created by `ensure_tracking_table`, which runs at the start of every `up`, `down`, and `status` call (after the advisory lock is acquired and setup SQL has run). The `CREATE TABLE IF NOT EXISTS` statement is idempotent — it is safe to run against a database that already has the table.

## What `down` does to the table

`revert` deletes the row for the rolled-back migration in the same transaction as the DOWN SQL. After a successful rollback, the migration is no longer in the tracking table and appears as pending in `status` output.

## What `status` shows

`status` reads the tracking table and compares it against the manifest. It produces two lists:

- **Applied:** rows from the tracking table — filename, version, batch, `applied_at`, `applied_by`, `execution_ms`.
- **Pending:** manifest entries with no corresponding tracking-table row.

Checksum verification runs during `status` — a drift error for any applied migration surfaces here, before any `up` or `down` is attempted.

## Atomic apply+track

The migration SQL and the tracking-table insert execute in a single transaction. If the migration SQL fails, the transaction is rolled back and no row is inserted. If the row insert fails (which should not happen under normal conditions), the migration SQL is also rolled back. There is no state where a migration ran but has no record, or a record exists for a migration that never ran.
