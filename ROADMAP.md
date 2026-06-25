# soma-schema Roadmap

## Vision

soma-schema aims to be a single migration tool that works across many database engines without changing how you write or organise migrations. The `MigrationDriver` trait in `src/driver.rs` is the seam: it defines six async operations — acquire lock, run setup, ensure tracking table, list applied, apply, revert — and each backend fills them in. The manifest-defined ordering, full-file checksum drift detection, run-scoped locking, and atomic apply+track are all implemented above the driver and require no changes when a new backend lands.

The migration file format (UP/DOWN SQL separated by `-- DOWN ==`) and the YAML manifest are neutral. Only the step that actually sends a payload to a database is backend-specific.

---

## Backend status

| Backend | Status | Notes |
| --- | --- | --- |
| PostgreSQL | ✅ Stable | `PostgresDriver` — full advisory lock, atomic apply+track |
| MySQL / MariaDB | 🔜 Planned | Phase 1 |
| SQLite | 🔜 Planned | Phase 1 |
| CockroachDB | 🔜 Planned | Phase 2 — validation via existing Postgres driver |
| mco-db | 🔜 Planned | Phase 2 — Postgres-wire engine; validation, not a new driver |
| SurrealDB | 🔜 Planned | Phase 3 — SurrealQL payload, transaction-based lock |
| MongoDB | 🔜 Planned | Phase 4 — structured op format; atomic ceiling noted below |

---

## Phases

### Phase 0 — PostgreSQL (done)

- Manifest-defined apply and rollback order via `migration-order.yaml`
- Full-file SHA-256 checksum drift detection (UP + DOWN together)
- Run-scoped advisory lock held via RAII guard for the entire `up`/`down` call
- Atomic apply+track: migration SQL and tracking-table row commit in one transaction

### Phase 1 — SQL family

**MySQL / MariaDB** and **SQLite** use the same plain-SQL UP/DOWN file format without changes. The work is implementing `MigrationDriver` for each:

- MySQL: `GET_LOCK` / `RELEASE_LOCK` as the advisory-lock equivalent; `sqlx` MySQL driver for query execution.
- SQLite: `BEGIN IMMEDIATE` as the lock primitive, or a lock file at the migrations root for multi-process scenarios. Single-connection model simplifies the two-connection pattern used by `PostgresDriver`.

The tracking table DDL is standard SQL; both engines support it without modification.

### Phase 2 — Postgres-wire-compatible engines

**CockroachDB** and **mco-db** (a from-scratch Rust engine that speaks the Postgres wire protocol, used elsewhere in this platform) should work through the existing `PostgresDriver` without a new implementation. Advisory locks behave differently on CockroachDB — this needs validation and may require a thin adapter or a flag, but the goal is zero additional driver code. The task here is integration testing and documentation, not building a new driver.

### Phase 3 — Multi-model / NewSQL

**SurrealDB** requires a new driver implementation:

- Migrations authored in SurrealQL (`.surql` files, UP/DOWN separated by the same `-- DOWN ==` delimiter).
- Locking via a dedicated lock record in SurrealDB or a transaction that blocks concurrent writers.
- Checksum and tracking table map to a SurrealDB table; `version`, `file`, `checksum`, `batch`, `applied_at` remain the same fields.
- The trait method `run_setup_sql` generalises to "run setup payload" — see the driver contract section below.

### Phase 4 — NoSQL

**MongoDB** is the first NoSQL target:

- Migrations are an ordered sequence of change operations — either a structured JSON op format or a JS aggregation script. The checksum is over the full file.
- Locking uses a lock document in a dedicated collection, updated with a `findAndModify` / `$set` + `$currentDate` pattern.

**Known ceiling:** MongoDB does not have multi-document transactions in all configurations (older servers, standalone deployments). This means apply+track cannot always be made fully atomic. The mitigation is: write migrations as idempotent operations; mark a migration as `pending` before executing and `applied` after; on the next run, a `pending` row triggers a conflict check before re-applying. This is explicitly a best-effort guarantee, not the atomic guarantee `PostgresDriver` provides.

> ponytail: the apply+track atomicity ceiling on engines without transactions is real and is not going to be papered over. Callers targeting those engines must write idempotent migrations. This limitation will be documented in the driver's error type and in the status output.

---

## Cross-cutting features (database-agnostic)

These are not tied to any backend phase. They work by changing the layer above `MigrationDriver`.

| Feature | Status | Notes |
| --- | --- | --- |
| `up --steps N` | 🔜 Planned | `down --steps N` already exists |
| Dry-run mode | 🔜 Planned | `--dry-run` flag; print SQL/payload without executing |
| `generate` / `new` command | 🔜 Planned | Scaffold a migration file + add the manifest entry |
| `verify` command | 🔜 Planned | Recompute checksums against applied rows without running migrations |
| `status --json` | 🔜 Planned | Machine-readable output; a `tools/explorer-data` JSON exporter already exists as a starting point |
| Squash / consolidate | 🔜 Planned | Collapse a range of applied migrations into a single file |

---

## Driver contract and how to add a backend

The full trait is in `src/driver.rs`. A new driver must provide:

1. **A locking primitive** that prevents concurrent `up`/`down` runs. The Postgres implementation uses a session-level advisory lock held on a dedicated connection. The equivalent on another engine can be anything exclusive for the duration of the call — `GET_LOCK`, a lock document, a transaction that blocks writers, or a file lock.

2. **Atomic apply+track** where the engine supports transactions. The `apply` method receives `up_sql` (already extracted from the file) and a `&Migration` reference; it must execute the SQL and insert the tracking row in a single transaction. `revert` does the same for DOWN SQL and tracking-row deletion. If the engine does not support multi-statement transactions, use the idempotent + pending-marker pattern described in Phase 4 above and document that you are doing so.

3. **Treat the migration payload as opaque.** The driver receives a `&str` (or, in a future version, a typed payload enum). It does not parse or validate the SQL — that is the caller's concern. For non-SQL backends, the payload will be whatever the format is for that engine.

**Planned driver contract change:** the current method names are SQL-flavoured (`run_setup_sql`, and the `up_sql` / `down_sql` parameter names in `apply` / `revert`). A v2 driver contract will replace these with payload-neutral names — `run_setup`, `apply(payload)`, `revert(payload)` — so that non-SQL backends fit without awkward naming. This will be introduced as a trait alias or a new trait version alongside the existing one, aiming not to break callers who only depend on `PostgresDriver`.

---

## Non-goals

- **Auto-generated schema-diff migrations.** Atlas generates migrations by diffing your desired schema against the current state. soma-schema does not do that and has no plans to. You write the migration; soma-schema runs it safely.
- **ORM coupling.** soma-schema has no opinion about your application framework and does not depend on one.
