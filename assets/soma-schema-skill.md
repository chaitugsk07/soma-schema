---
name: soma-schema
description: >
  How to adopt and operate the soma-schema migration tool (Rust, Postgres) correctly
  in any repo: dependency setup, the migrations/ + migration-order.yaml contract,
  UP/DOWN files, run-at-startup vs CLI, and the non-negotiable invariants (never edit
  applied files, manifest-complete ordering, FK-safe DOWN, one schema + unique advisory
  lock key per service, pool >= 2). The runner contract; pairs with /db-standards for
  SQL content. Invoke with /soma-schema.
metadata:
  author: Sri / Kreesalis
  version: "1.0.0"
---

# soma-schema — Migration Runner Rules

Rules for wiring and operating the soma-schema migration tool in any soma service. This skill covers the runner contract (dependency, directory structure, library/CLI API, invariants). It does **not** govern SQL content — use `/db-standards` for that.

The per-repo source of truth is `soma-schema/CONSUMING.md`. This skill is the global mirror for agents working in any repo.

---

## When to apply

- Designing or wiring soma-schema into a new service.
- Writing or reviewing any migration file (`.sql`) or `migration-order.yaml`.
- Debugging `ChecksumDrift`, `OrphanMigration`, `MissingFile`, or `PoolTooSmall` errors.
- Any `up`/`down`/`status` workflow question.

---

## Cargo dependency

The `cli` feature is on by default and pulls in `clap`. Services embedding the library should disable it:

```toml
# Sibling clone (active dev):
soma-schema = { path = "../soma-schema", default-features = false }

# Pinned git tag:
soma-schema = { git = "https://github.com/chaitugsk07/soma-schema", tag = "v0.1.0", default-features = false }

# crates.io (once published):
soma-schema = { version = "0.1", default-features = false }
```

Keep `default-features = true` only if you also need the `soma-schema` CLI binary built from this dep. Embedding the library at startup is the norm; in that case always set `default-features = false`.

---

## migrations/ directory contract

```text
migrations/
  migration-order.yaml      # authoritative manifest — apply AND rollback order
  00_setup/                 # idempotent bootstrap; runs every up(), never tracked
    01_schema.sql
  01_migrated/
    1/                      # integer version folder, sorted numerically
      20260101_01_init.sql
  02_inprogress/            # optional staging area for in-flight work
```

### SQL file format

```sql
-- UP section
CREATE TABLE soma_iam.roles (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), name TEXT NOT NULL);

-- DOWN ==
DROP TABLE IF EXISTS soma_iam.roles;
```

The delimiter is the line that trims to exactly `-- DOWN ==`. Checksum covers the entire file — UP and DOWN together. Any post-deploy edit triggers `ChecksumDrift`.

### migration-order.yaml

```yaml
manifest_version: 1
versions:
  - version: 1
    description: "Initial IAM schema"
    migrations:
      - file: "20260101_01_init.sql"
        created: "2026-01-01"
        author: "you"
        why: "Core tables"
```

- `manifest_version` must be `1`.
- `file` is a bare filename — no path separators, must end in `.sql`.
- File on disk, absent from manifest → `OrphanMigration`.
- Manifest entry, no file → `MissingFile`.
- Version folders sorted numerically. `00_setup/` files never listed here.

---

## Library integration (run at startup)

```rust
use soma_schema::{Migrator, PostgresConfig, PostgresDriver};

let driver = PostgresDriver::new(pool.clone(), PostgresConfig {
    schema: Some("soma_iam".into()),
    advisory_lock_key: 0x_50A_1A33, // unique per service
    ..Default::default()             // table: "00_schema_migrations"
})?;
Migrator::from_root("migrations").up(&driver).await?;
```

Crate-root exports: `Migrator`, `PostgresDriver`, `PostgresConfig`, `MigrationDriver`, `MigrationStatus`, `AppliedMigration`, `PendingMigration`, `Error`, `Result`.

---

## CLI usage

```sh
soma-schema --database-url "$DATABASE_URL" --schema soma_iam --migrations migrations up
soma-schema --database-url "$DATABASE_URL" --schema soma_iam --migrations migrations status
soma-schema --database-url "$DATABASE_URL" --schema soma_iam --migrations migrations down --steps 1
```

---

## Non-negotiable invariants

1. **Never edit an applied migration file.** Any change — even a comment — triggers `ChecksumDrift`. Write a new migration instead.

2. **Every new `.sql` must be listed in `migration-order.yaml`** in the correct version block, in apply order.

3. **Write a DOWN section for every migration unless genuinely irreversible.** DOWN must undo UP in FK-safe reverse order. Rollback order = reverse of manifest position, so manifest order must be FK-correct going forward.

4. **Seeds are idempotent migrations.** UP uses `ON CONFLICT DO NOTHING` so re-running after rollback is safe.

5. **One schema per service.** Set `PostgresConfig.schema`; the tracking table lives in that schema. `00_setup/` must `CREATE SCHEMA IF NOT EXISTS` it.

6. **Pool needs `max_connections >= 2`.** One connection holds the advisory lock for the full `up`/`down` call. `PostgresDriver::new` returns `PoolTooSmall` if the pool cannot provide two.

7. **Unique `advisory_lock_key` per service when services share one Postgres database.** Advisory locks are database-global by key. Default is `918273645`; pick a distinct `i64` constant per service and document it.

8. **`00_setup/` SQL must be idempotent.** `CREATE SCHEMA IF NOT EXISTS`, `CREATE OR REPLACE FUNCTION`, etc. Runs on every `up()`, never tracked.

9. **SQL content follows `/db-standards`.** soma-schema is the runner; db-standards governs what SQL may contain — naming, types, `pgcrypto` only, no exotic Postgres features for mco-db portability.

---

## Adding a migration — the agent loop

1. Pick or create the correct version folder under `01_migrated/` (or `02_inprogress/`).
2. Create `<YYYYMMDD>_<NN>_<name>.sql` with UP + `-- DOWN ==` + DOWN.
3. Add an entry to `migration-order.yaml` with `created`, `author`, and `why`.
4. Run `status` to confirm the file appears as pending.
5. Run `up` (or start the service) to apply it.
6. Never touch the file again once it has been applied to any environment.

---

## Pairs with

- `/db-standards` — SQL content rules (naming, types, allowed features, EAV conventions).
- Per-repo `soma-schema/CONSUMING.md` — project-local source of truth with service-specific examples.
