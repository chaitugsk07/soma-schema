+++
title = "Roadmap"
description = "Planned backends and cross-cutting features — phases and current status."
weight = 80
+++

soma-schema is Postgres-only today. The `MigrationDriver` trait is the seam: it defines six async operations (acquire lock, run setup, ensure tracking table, list applied, apply, revert), and each backend fills them in. The manifest ordering, full-file checksum detection, run-scoped locking, and atomic apply+track are all implemented above the driver and require no changes when a new backend lands.

The full `ROADMAP.md` is in the [GitHub repository](https://github.com/chaitugsk07/soma-schema/blob/main/ROADMAP.md).

## Backend status

| Backend | Status | Phase |
|---|---|---|
| PostgreSQL | Stable | 0 (done) |
| MySQL / MariaDB | Planned | 1 |
| SQLite | Planned | 1 |
| CockroachDB | Planned | 2 |
| mco-db | Planned | 2 |
| SurrealDB | Planned | 3 |
| MongoDB | Planned | 4 |

## Phase 0 — PostgreSQL (done)

- Manifest-defined apply and rollback order via `migration-order.yaml`
- Full-file SHA-256 checksum drift detection (UP + DOWN together)
- Run-scoped advisory lock held via RAII guard for the entire `up`/`down` call
- Atomic apply+track: migration SQL and tracking-table row commit in one transaction

## Phase 1 — SQL family

MySQL, MariaDB, and SQLite use the same plain-SQL UP/DOWN file format without changes. The work is implementing `MigrationDriver` for each engine:

- **MySQL:** `GET_LOCK` / `RELEASE_LOCK` as the advisory-lock equivalent; sqlx MySQL driver for query execution.
- **SQLite:** `BEGIN IMMEDIATE` as the lock primitive, or a lock file at the migrations root for multi-process scenarios. The single-connection model simplifies the two-connection pattern used by `PostgresDriver`.

The tracking table DDL is standard SQL; both engines support it without modification.

## Phase 2 — Postgres-wire-compatible engines

CockroachDB and mco-db (a Rust engine that speaks the Postgres wire protocol) should work through the existing `PostgresDriver` without a new implementation. Advisory lock semantics differ on CockroachDB — this needs validation and may require a thin adapter, but the goal is zero additional driver code. The work is integration testing and documentation, not a new driver.

## Phase 3 — Multi-model / NewSQL

SurrealDB requires a new driver:

- Migrations authored in SurrealQL (`.surql` files, UP/DOWN separated by the same `-- DOWN ==` delimiter).
- Locking via a dedicated lock record or a blocking transaction.
- Tracking table maps to a SurrealDB table; same columns.

## Phase 4 — NoSQL

MongoDB is the first NoSQL target. Migrations are an ordered sequence of change operations. Locking uses a lock document in a dedicated collection.

**Known ceiling:** MongoDB does not have multi-document transactions in all configurations. Apply+track cannot always be made fully atomic. The mitigation is idempotent migrations with a pending-marker pattern. This ceiling is explicitly documented — it is not papered over.

## Cross-cutting features

These are database-agnostic and do not require a new backend:

| Feature | Status |
|---|---|
| `up --steps N` | Planned (`down --steps N` already exists) |
| `--dry-run` flag | Planned |
| `generate` / `new` command | Planned |
| `verify` command | Planned |
| `status --json` | Planned |
| Squash / consolidate | Planned |

## Non-goals

soma-schema does not generate SQL by diffing schemas (use [Atlas](https://atlasgo.io/) for that). It does not couple to any ORM. It runs the SQL you write.
