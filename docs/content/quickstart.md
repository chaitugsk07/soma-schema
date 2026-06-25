+++
title = "Quickstart"
description = "The 60-second CLI flow — scaffold, write, apply, check, roll back."
weight = 20
+++

This walks through the full CLI workflow from scratch. It assumes soma-schema is [installed](@/installation.md) and you have a Postgres database reachable via a connection URL.

## 1. Scaffold the migrations directory

```sh
soma-schema init migrations/
```

This creates the directory structure:

```text
migrations/
  migration-order.yaml
  00_setup/
    01_schema.sql
  01_migrated/
    1/
      (empty — add your first migration here)
```

## 2. Write your setup SQL

Edit `migrations/00_setup/01_schema.sql`. This file runs before every `up` and is never tracked — it should be idempotent:

```sql
CREATE SCHEMA IF NOT EXISTS myapp;
```

Files in `00_setup/` are never listed in `migration-order.yaml` and are never orphan-flagged.

## 3. Write your first migration

Create `migrations/01_migrated/1/20260101_01_init.sql`:

```sql
CREATE TABLE myapp.users (
    id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL
);

-- DOWN ==
DROP TABLE IF EXISTS myapp.users;
```

The delimiter must be exactly `-- DOWN ==` (trimmed). Everything before it is the UP section; everything after is DOWN.

## 4. Register it in the manifest

Edit `migrations/migration-order.yaml`:

```yaml
manifest_version: 1
versions:
  - version: 1
    description: "Initial schema"
    migrations:
      - file: "20260101_01_init.sql"
        created: "2026-01-01"
        author: "you"
        why: "Create users table"
```

Every `.sql` file under a version folder must have an entry here. A file on disk but missing from the manifest causes an `OrphanMigration` error.

## 5. Apply

```sh
soma-schema --database-url "$DATABASE_URL" --migrations migrations/ up
```

`DATABASE_URL` can also be set as an environment variable; `--database-url` overrides it.

## 6. Check status

```sh
soma-schema --database-url "$DATABASE_URL" --migrations migrations/ status
```

Output shows applied migrations (with applied-at timestamp and batch number) and pending migrations.

## 7. Roll back

```sh
# Roll back the last migration.
soma-schema --database-url "$DATABASE_URL" --migrations migrations/ down

# Roll back the last 3 migrations.
soma-schema --database-url "$DATABASE_URL" --migrations migrations/ down --steps 3
```

Rollback order is the exact reverse of manifest position — not filename sort. This makes it safe across FK relationships without naming tricks.

## Next steps

- [Migration file format](@/migration-file-format.md) — delimiter, UP/DOWN rules, seeds
- [Your first migration](@/first-migration.md) — step-by-step tutorial with more detail
- [CLI reference](@/cli-reference.md) — all flags
