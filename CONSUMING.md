# Using soma-schema in Another Repo

This is the portable source of truth for how a soma service (soma-iam, soma-vault, etc.) adopts soma-schema for its database migrations. Keep this file checked in alongside the service's `migrations/` directory.

---

## Cargo dependency

soma-schema's `cli` feature is on by default and pulls in `clap`. A consumer using it as an embedded library should turn that off:

```toml
# Active development (sibling clone) — library use, no CLI:
soma-schema = { path = "../soma-schema", default-features = false }

# Stable pin via git tag:
soma-schema = { git = "https://github.com/chaitugsk07/soma-schema", tag = "v0.1.0", default-features = false }

# Once published to crates.io:
soma-schema = { version = "0.1", default-features = false }
```

Keep `default-features = true` (or omit it entirely) only if you also want the `soma-schema` CLI binary built from this dependency. Most services embed the library and run migrations at startup — `default-features = false` is the norm.

---

## The migrations/ directory contract

Create `migrations/` at the repo root. Fastest path: `soma-schema init migrations/` (if the CLI is installed) scaffolds the structure for you.

```text
migrations/
  migration-order.yaml      # authoritative manifest — defines apply AND rollback order
  00_setup/                 # idempotent bootstrap; runs every up(), never tracked
    01_schema.sql
  01_migrated/
    1/                      # version folder; integer name, sorted numerically
      20260101_01_init.sql
      20260101_02_seed.sql
  02_inprogress/            # optional staging area for in-flight work
```

### SQL file format

Each `.sql` file holds both directions:

```sql
-- UP section (everything before the delimiter)
CREATE TABLE soma_iam.roles (
    id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL
);

-- DOWN ==
DROP TABLE IF EXISTS soma_iam.roles;
```

The delimiter line must trim to exactly `-- DOWN ==`. Everything before it is the UP section; everything after is DOWN. The checksum is SHA-256 of the entire file — UP and DOWN together.

### migration-order.yaml

Every `.sql` file under a version folder must be listed here. This file is the single source of truth for apply order (top to bottom) and rollback order (exact reverse of manifest position).

```yaml
manifest_version: 1
versions:
  - version: 1
    description: "Initial IAM schema"
    migrations:
      - file: "20260101_01_init.sql"
        created: "2026-01-01"
        author: "you"
        why: "Core roles and permissions tables"
      - file: "20260101_02_seed.sql"
        created: "2026-01-01"
        author: "you"
        why: "Seed built-in roles"
```

Rules:
- `manifest_version` must be `1`.
- `file` is a bare filename — no path separators, must end in `.sql`.
- A file on disk but absent from the manifest → `OrphanMigration` error.
- A manifest entry with no corresponding file → `MissingFile` error.
- Version folders are sorted numerically (`10` comes after `2`).
- `00_setup/` files are never listed here and are never orphan-flagged.

---

## Library integration (run at service startup — the common case)

```rust
use soma_schema::{Migrator, PostgresConfig, PostgresDriver};

// Pool must allow >= 2 connections (one is held for the advisory lock).
let driver = PostgresDriver::new(pool.clone(), PostgresConfig {
    schema: Some("soma_iam".into()),
    advisory_lock_key: 0x_50A_1A33, // UNIQUE per service if services share one database
    ..Default::default()
})?;
Migrator::from_root("migrations").up(&driver).await?;
```

Public API at the crate root:

| Symbol | Description |
| --- | --- |
| `Migrator` | Owns the migrations root; drives `up`, `down`, `status`, `scaffold` |
| `PostgresDriver` | Postgres implementation of `MigrationDriver` |
| `PostgresConfig` | Driver config: `schema`, `table`, `advisory_lock_key` |
| `MigrationDriver` | Trait to implement for additional database backends |
| `MigrationStatus` | Return type of `status()`: applied + pending lists |
| `AppliedMigration` | A row from the tracking table |
| `PendingMigration` | A manifest entry not yet applied |
| `Error`, `Result` | Crate error type and alias |

`PostgresConfig` defaults: `table = "00_schema_migrations"`, `advisory_lock_key = 918273645`. Always override `schema` and `advisory_lock_key` in a real service.

---

## CLI usage (deploy-pipeline alternative)

```sh
soma-schema --database-url "$DATABASE_URL" --schema soma_iam --migrations migrations up
soma-schema --database-url "$DATABASE_URL" --schema soma_iam --migrations migrations status
soma-schema --database-url "$DATABASE_URL" --schema soma_iam --migrations migrations down --steps 1
```

`DATABASE_URL` can also be set as an environment variable; `--database-url` overrides it.

---

## Non-negotiable invariants

1. **Never edit an applied migration file.** The checksum is SHA-256 of the whole file (UP + DOWN). Any edit — even a comment — triggers `ChecksumDrift` on the next command. To change deployed schema, write a new migration.

2. **Every new `.sql` must be in `migration-order.yaml`.** Add it to the correct version block in apply order. A file not in the manifest → `OrphanMigration`; a manifest entry with no file → `MissingFile`.

3. **Write a DOWN section for every migration unless it is genuinely irreversible.** DOWN must undo UP in FK-safe reverse order (drop children before parents). Rollback order is the reverse of manifest position, so manifest order must be FK-correct going forward.

4. **Seeds are idempotent migrations.** UP uses `ON CONFLICT DO NOTHING` (or equivalent) so re-running after a rollback is safe.

5. **One schema per service.** Set `PostgresConfig.schema` (e.g. `soma_iam`); the `00_schema_migrations` tracking table lives in that schema. `00_setup/` must `CREATE SCHEMA IF NOT EXISTS` it before anything else.

6. **Pool needs `max_connections >= 2`.** One connection is held for the run-scoped advisory lock; `PostgresDriver::new` returns `PoolTooSmall` if the pool cannot provide two connections.

7. **Unique `advisory_lock_key` per service when multiple services share one Postgres database.** Advisory locks are database-global by key. Two services using the default key would serialize against each other. Pick a distinct constant per service and document it in its `PostgresConfig` call. (The key is a single `i64`; just pick a unique hex constant and leave a comment naming the service.)

8. **`00_setup/` SQL must be idempotent.** Use `CREATE SCHEMA IF NOT EXISTS`, `CREATE OR REPLACE FUNCTION`, and similar. This directory runs on every `up()` and is never tracked.

9. **SQL content follows the global db-standards skill.** soma-schema is the runner; `/db-standards` governs what the SQL may contain — naming, types, `pgcrypto` only, no exotic Postgres features for mco-db portability. Cross-reference it before writing migration SQL.

---

## Adding a migration — the agent loop

1. Pick or create the correct version folder under `01_migrated/` (or `02_inprogress/` for in-flight work).
2. Create `<YYYYMMDD>_<NN>_<descriptive_name>.sql` with a UP section, the `-- DOWN ==` delimiter, and a DOWN section.
3. Add an entry to `migration-order.yaml` in the right version block with `created`, `author`, and `why` fields.
4. Run `soma-schema ... status` to confirm the new file appears as pending.
5. Run `soma-schema ... up` (or let the service start) to apply it.
6. Never touch the file again once it has been applied to any environment.

---

## CLAUDE.md block for the consumer repo

Paste this into the consumer repo's own `CLAUDE.md`:

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

Adding a migration:
- Create <YYYYMMDD>_<NN>_<name>.sql with UP + "-- DOWN ==" + DOWN.
- Add it to migration-order.yaml (created/author/why).
- Run status to confirm pending, then up to apply.
- Never touch the file again once applied to any environment.
```
