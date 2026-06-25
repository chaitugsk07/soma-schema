# soma-schema Roadmap

## Direction

Postgres deep, SQLite next, everything else via the `MigrationDriver` interface.

PostgreSQL is stable and the primary focus. SQLite is the committed next backend — the same plain-SQL UP/DOWN format applies unchanged, and it is the natural fit for projects that start on SQLite and graduate to Postgres. Beyond those two, the `MigrationDriver` trait is the extension point: any backend that can run a query, hold a lock, and commit atomically can be wired in without touching the core.

---

## Databases

### Stable

| Backend | Notes |
| --- | --- |
| **PostgreSQL** | `PostgresDriver` — full advisory lock, atomic apply+track, full-file drift detection. Continuing to deepen. |

### Committed (next)

| Backend | Notes |
| --- | --- |
| **SQLite** | Same plain-SQL UP/DOWN format, no changes to migration files. Lock primitive: `BEGIN IMMEDIATE` (single-writer) or a lock file at the migrations root for multi-process scenarios. Single-connection model simplifies the two-connection pattern `PostgresDriver` uses. Primary use case: SQLite-now → Postgres-later. A project uses soma-schema migrations from day one on SQLite, then switches the backend without touching any migration file. |

### Planned

| Backend | Notes |
| --- | --- |
| **MySQL / MariaDB** | Advisory-lock equivalent via `GET_LOCK` / `RELEASE_LOCK`. sqlx MySQL driver for execution. Tracking table DDL is standard SQL — no engine-specific changes needed. |
| **CockroachDB** | Largely works through the existing `PostgresDriver` today — it speaks the Postgres wire protocol. Advisory lock semantics differ and need validation. The goal is zero additional driver code; the work is integration testing and documentation. |

### Exploratory — multi-model vision

These backends are further out and carry honest caveats. There is no commitment to a timeline.

| Backend | Notes |
| --- | --- |
| **SurrealDB** | Migrations authored in SurrealQL (`.surql` files, UP/DOWN separated by the same `-- DOWN ==` delimiter). New driver required. Locking via a dedicated lock record or a blocking transaction. |
| **MongoDB** | Ordered, idempotent change operations as the migration payload. Locking via a lock document in a dedicated collection. **Honest ceiling:** without multi-document transactions in all configurations, atomic apply+track cannot always be guaranteed. The mitigation is idempotent migrations with a pending-marker pattern. This is documented, not papered over. |
| **DuckDB** | Analytics workloads that need schema versioning. Single-writer model simplifies locking. |

### Community-welcome

Any backend is implementable today by satisfying the `MigrationDriver` trait in [`src/driver.rs`](src/driver.rs). The manifest ordering, full-file checksum detection, run-scoped locking, and atomic apply+track all live above the driver and are reused automatically. Open an issue to coordinate before starting a new backend — it avoids duplicate work and gets you early feedback on the implementation plan.

---

## What's done (PostgreSQL)

- Manifest-defined apply and rollback order via `migration-order.yaml`
- Full-file SHA-256 checksum drift detection (UP + DOWN together)
- Run-scoped advisory lock held via RAII guard for the entire `up`/`down` call
- Atomic apply+track: migration SQL and tracking-table row commit in one transaction
- Visual explorer: self-contained HTML page, no database connection needed
- `AppliedMigration::new()` — public constructor so external `MigrationDriver` implementations can construct tracking rows without depending on internals
- `status` surfacing `drift_errors` without aborting — usable as a pre-flight check in CI

---

## Features still needed (database-agnostic)

These are cross-cutting changes that live above `MigrationDriver` and benefit every backend once done.

| Feature | Notes |
| --- | --- |
| `--dry-run` | Print the SQL that would run without executing it. Works for both `up` and `down`. |
| `up --steps N` | Apply exactly N pending migrations. `down --steps N` already exists. |
| `status --json` | Machine-readable status output for CI pipelines. A JSON exporter already exists in the explorer path — this surfaces it on `status`. Pairs with the existing `drift_errors` field. |
| Advisory-lock timeout / `--lock-timeout` | Non-blocking acquire with a clear "another migration is running" error instead of hanging indefinitely. |
| `verify` | Re-check all checksums and tracking-table integrity without applying or reverting anything. Useful as a standalone CI gate after a deploy. |
| `repair` / `baseline` | Adopt an existing database (stamp it as applied without running the SQL) or fix a corrupted tracking table after manual intervention. |
| `new` / `generate` | Scaffold a new migration file with the correct filename convention and add the manifest entry automatically. Removes the main source of user error. |
| Migration squash / consolidate | Collapse a range of applied migrations into a single file for projects with very long histories. Requires careful checksum handling — the new consolidated file becomes the source of truth. |
| Structured timing / observability | `execution_ms` is already recorded in the tracking table. Surface it in `status` output and the JSON export. |

---

## Driver contract and how to add a backend

The full trait is in [`src/driver.rs`](src/driver.rs). A new driver must provide three things:

1. **A locking primitive** that prevents concurrent `up`/`down` runs for the duration of the call. `PostgresDriver` uses a session-level advisory lock on a dedicated connection. The equivalent on another engine can be anything exclusive — `GET_LOCK`, a lock document, a transaction that blocks writers, or a file lock.

2. **Atomic apply+track** where the engine supports transactions. `apply` receives `up_sql` (already extracted from the file) and a `&Migration` reference; it must execute the SQL and insert the tracking row in a single transaction. `revert` does the same for DOWN SQL and tracking-row deletion. Where true atomicity is not available (some NoSQL engines), the implementation must be idempotent and document the ceiling.

3. **Treat the migration payload as opaque.** The driver receives a `&str`. It does not parse or validate the SQL — that is the caller's concern.

---

## Non-goals

- **Auto-generated schema-diff migrations.** [Atlas](https://atlasgo.io/) generates migrations by diffing your desired schema against the current state. soma-schema does not do that and has no plans to. You write the migration; soma-schema runs it safely.
- **ORM coupling.** soma-schema has no opinion about your application framework and does not depend on one.
