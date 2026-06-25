# soma-schema Roadmap

## Direction

Postgres deep, SQLite next, everything else via the `MigrationDriver` interface.

PostgreSQL is stable and the primary focus. SQLite is the committed next backend — the same plain-SQL UP/DOWN format applies unchanged, and it's the natural fit for projects that start on SQLite and graduate to Postgres. Beyond that, the `MigrationDriver` trait is the extension point: any backend that can run SQL, hold a lock, and commit atomically can be wired in.

---

## Backend status

| Backend | Status | Notes |
| --- | --- | --- |
| PostgreSQL | Stable | `PostgresDriver` — full advisory lock, atomic apply+track |
| SQLite | Planned | Same plain-SQL format; `BEGIN IMMEDIATE` as lock primitive |

Community-contributed backends are welcome. MySQL/MariaDB are the most natural candidates — `GET_LOCK` / `RELEASE_LOCK` as the lock primitive, `sqlx` MySQL driver for execution. Open an issue to coordinate before starting.

---

## What's done (PostgreSQL)

- Manifest-defined apply and rollback order via `migration-order.yaml`
- Full-file SHA-256 checksum drift detection (UP + DOWN together)
- Run-scoped advisory lock held via RAII guard for the entire `up`/`down` call
- Atomic apply+track: migration SQL and tracking-table row commit in one transaction
- Visual explorer: self-contained HTML page, no database needed

## SQLite (next)

SQLite uses the same plain-SQL UP/DOWN file format without changes. The work is implementing `MigrationDriver`:

- `BEGIN IMMEDIATE` as the lock primitive (or a lock file at the migrations root for multi-process scenarios).
- Single-connection model simplifies the two-connection pattern `PostgresDriver` uses.
- The tracking table DDL is standard SQL; SQLite supports it without modification.

The primary use case is SQLite-now → Postgres-later: a project starts on SQLite, uses soma-schema migrations from day one, and later switches the backend without touching the migration files.

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

2. **Atomic apply+track** where the engine supports transactions. The `apply` method receives `up_sql` (already extracted from the file) and a `&Migration` reference; it must execute the SQL and insert the tracking row in a single transaction. `revert` does the same for DOWN SQL and tracking-row deletion.

3. **Treat the migration payload as opaque.** The driver receives a `&str`. It does not parse or validate the SQL — that is the caller's concern.

---

## Non-goals

- **Auto-generated schema-diff migrations.** Atlas generates migrations by diffing your desired schema against the current state. soma-schema does not do that and has no plans to. You write the migration; soma-schema runs it safely.
- **ORM coupling.** soma-schema has no opinion about your application framework and does not depend on one.
