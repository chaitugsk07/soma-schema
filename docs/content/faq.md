+++
title = "FAQ"
description = "Common questions about soma-schema."
weight = 90
+++

## Why not just use sqlx migrate?

sqlx's built-in migrator is a natural starting point if you are already using sqlx. It has two gaps that soma-schema addresses:

1. **Checksum coverage.** sqlx hashes only the UP section. If someone edits the DOWN section of a deployed migration, the drift is invisible until a rollback is attempted. soma-schema checksums the entire file — UP and DOWN — so any edit is caught the next time any command runs.

2. **Locking scope.** sqlx acquires an advisory lock per migration rather than for the full run. Between migrations, another runner could start. soma-schema holds the lock for the entire `up`/`down` call.

If neither of those matters for your use case, sqlx migrate is simpler and already a dependency.

## Why a YAML manifest instead of filename ordering?

Filename ordering looks simple but breaks down at rollback time. If `02_add_widget.sql` creates a table that `01_add_gadget.sql`'s FK depends on, rolling back in reverse filename order (`02`, then `01`) is wrong — you need to drop the FK target last. With a manifest, you explicitly list `01_add_gadget.sql` after `02_add_widget.sql` and rollback automatically reverses that: drop `01` first, then `02`. No naming tricks required.

## Why is the tracking table named `00_schema_migrations`?

The `00_` prefix causes it to sort before application tables in most database GUI tools — it appears at the top of the table list. The default name can be overridden via `PostgresConfig.table` or the `--table` CLI flag.

## Can I rename a migration file after applying it?

No. The tracking table stores the filename and a checksum keyed to that filename. Renaming would break the connection between the record and the file, and soma-schema would see it as an orphan + a missing file at the same time. Write a new migration if you need to change what a migration does.

## What happens if a migration fails halfway through?

The `apply` call runs the migration SQL and the tracking-table insert in a single transaction. If anything fails, the transaction is rolled back — the database is exactly as it was before the migration started. The failed migration is not recorded in the tracking table.

Note: this guarantee holds only for transactional DDL. Some Postgres DDL (like `CREATE INDEX CONCURRENTLY`) cannot run inside a transaction. If your migration uses non-transactional DDL and it fails partway through, you need to write the DOWN section carefully enough to clean up whatever was partially completed.

## Can I run migrations from multiple services against the same database?

Yes, provided each service uses a distinct `advisory_lock_key` and a distinct `schema`. Advisory locks are keyed database-globally. Two services with the same key would serialize against each other even if they touch different schemas. Pick a unique `i64` constant per service and document it in the `PostgresConfig` call.

## Do I need to run `00_setup/` SQL manually?

No. soma-schema runs all files in `00_setup/` automatically at the start of every `up` call, before checking for pending migrations. They never appear in the tracking table and are never orphan-flagged. They must be idempotent — use `CREATE SCHEMA IF NOT EXISTS`, `CREATE OR REPLACE FUNCTION`, and so on.

## What databases does soma-schema support?

PostgreSQL only, today. MySQL, SQLite, CockroachDB, mco-db, SurrealDB, and MongoDB are on the [roadmap](@/roadmap.md).

## Can I use soma-schema without Rust in my stack?

Yes — the CLI binary is a standalone binary. You can use it in a deploy pipeline without any Rust code in your application. That said, the library form (embedded at startup) is the main intended use for soma services.

## Where is the full competitor comparison?

In [`docs/competitor-analysis.md`](https://github.com/chaitugsk07/soma-schema/blob/main/docs/competitor-analysis.md) in the GitHub repository. It covers sqlx migrate, refinery, diesel_migrations, dbmate, golang-migrate, goose, Atlas, Flyway, Liquibase, sqitch, Alembic — with a "when NOT to choose soma-schema" section.
