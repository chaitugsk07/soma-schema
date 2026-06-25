+++
title = "Roadmap"
description = "Planned backends and cross-cutting features — tiers and current status."
weight = 80
+++

soma-schema is Postgres-stable today. The `MigrationDriver` trait is the seam: it defines six async operations (acquire lock, run setup, ensure tracking table, list applied, apply, revert), and each backend fills them in. The manifest ordering, full-file checksum detection, run-scoped locking, and atomic apply+track are all implemented above the driver and require no changes when a new backend lands.

---

## Databases

### Stable

**PostgreSQL** — `PostgresDriver` ships with the crate. Full advisory lock held via RAII guard for the entire `up`/`down` call, atomic apply+track in one transaction, full-file SHA-256 drift detection. This is the ongoing primary focus.

### Committed — next

**SQLite** — the SQLite-now → Postgres-later path. The UP/DOWN file format is identical; no migration files change when you switch backends. Lock primitive: `BEGIN IMMEDIATE` for single-writer safety, or a lock file at the migrations root for multi-process scenarios. The single-connection model simplifies the two-connection pattern `PostgresDriver` uses.

### Planned

**MySQL / MariaDB** — `GET_LOCK` / `RELEASE_LOCK` as the advisory-lock equivalent; sqlx MySQL driver for execution. Tracking table DDL is standard SQL.

**CockroachDB** — speaks the Postgres wire protocol, so it largely works through `PostgresDriver` today. Advisory lock semantics differ and need validation. The goal is zero additional driver code; the work is integration testing and documentation.

### Exploratory — multi-model vision

No timeline commitment on these. Honest about the constraints.

**SurrealDB** — migrations authored in SurrealQL (`.surql` files, UP/DOWN separated by the same `-- DOWN ==` delimiter). New driver required. Locking via a dedicated lock record or a blocking transaction.

**MongoDB** — ordered, idempotent change operations as the migration payload. Locking via a lock document in a dedicated collection. **Known ceiling:** without multi-document transactions in all configurations, atomic apply+track cannot always be guaranteed. The mitigation is idempotent migrations with a pending-marker pattern — explicitly documented, not papered over.

**DuckDB** — analytics workloads that need schema versioning. Single-writer model simplifies locking.

### Community-welcome

Any backend is implementable today by satisfying `MigrationDriver` in [`src/driver.rs`](https://github.com/chaitugsk07/soma-schema/blob/main/src/driver.rs). The manifest ordering, checksum detection, locking, and atomic apply+track are all reused automatically. Open an issue to coordinate before starting.

---

## What's done (PostgreSQL)

- Manifest-defined apply and rollback order via `migration-order.yaml`
- Full-file SHA-256 checksum drift detection (UP + DOWN together)
- Run-scoped advisory lock via RAII guard for the entire `up`/`down` call
- Atomic apply+track: migration SQL and tracking-table row commit in one transaction
- Visual explorer: self-contained HTML page, no database connection needed
- `AppliedMigration::new()` — public constructor for external `MigrationDriver` implementations
- `status` surfacing `drift_errors` without aborting — usable as a CI pre-flight check

---

## Features still needed (database-agnostic)

These live above `MigrationDriver` and benefit every backend once done.

| Feature | Notes |
|---|---|
| `--dry-run` | Print the SQL that would run without executing it — `up` and `down` |
| `up --steps N` | Apply exactly N pending migrations (`down --steps N` already exists) |
| `status --json` | Machine-readable output for CI; pairs with the existing `drift_errors` field |
| `--lock-timeout` | Non-blocking advisory-lock acquire with a clear "migration already running" error |
| `verify` | Re-check all checksums and tracking-table integrity without applying or reverting |
| `repair` / `baseline` | Adopt an existing database or fix a corrupted tracking table after manual intervention |
| `new` / `generate` | Scaffold a migration file with the correct naming convention and add the manifest entry |
| Squash / consolidate | Collapse a range of applied migrations into a single file for long-lived projects |
| Structured timing in `status` | `execution_ms` is already recorded per migration; surface it in `status` and JSON output |

---

## Driver contract

The full trait is in [`src/driver.rs`](https://github.com/chaitugsk07/soma-schema/blob/main/src/driver.rs). A new driver must provide:

1. **A locking primitive** exclusive for the duration of the call — `GET_LOCK`, a lock document, a blocking transaction, or a file lock.
2. **Atomic apply+track** where the engine supports transactions. Where true atomicity is not available, the implementation must be idempotent and document the ceiling.
3. **Opaque payload handling.** The driver receives a `&str` and executes it. It does not parse or validate SQL.

---

## Non-goals

soma-schema does not generate SQL by diffing schemas (use [Atlas](https://atlasgo.io/) for that). It does not couple to any ORM. It runs the SQL you write.
