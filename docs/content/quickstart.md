+++
title = "Quickstart"
description = "One command to install, scaffold, and run your first migration."
weight = 20
+++

This walks through the full CLI workflow from scratch. You need a Postgres database reachable via a connection URL.

## The one-liner

```sh
cargo install soma-schema && soma-schema init
```

That is the complete setup. It installs the CLI and immediately scaffolds your project.

### What `init` creates

```text
.                          <- repo root
+-- AGENTS.md              <- agent-rules so any AI agent follows the conventions
+-- migrations/
    +-- migration-order.yaml
    +-- 00_setup/
    |   +-- 01_schema.sql  <- idempotent bootstrap (edit to add your schema)
    +-- 01_migrated/
        +-- 1/
            +-- 20260101_01_example.sql  <- runnable example, already in the manifest
```

### Init pipeline

```text
cargo install soma-schema
        |
        v
soma-schema init
        |
        +---> migrations/
        |       migration-order.yaml
        |       00_setup/01_schema.sql
        |       01_migrated/1/<date>_01_example.sql
        |
        +---> AGENTS.md  (agent rules at repo root)
        |
        v
set DATABASE_URL
        |
        v
soma-schema up  (applies the example migration immediately)
        |
        v
soma-schema explorer  (visual UI, no DB needed)
```

## Step by step

### 1. Install and scaffold

```sh
cargo install soma-schema && soma-schema init
```

To also install the `/soma-schema` Claude skill and open the explorer immediately:

```sh
soma-schema init --skill --explore
```

To write rules for a specific tool instead of `AGENTS.md`:

```sh
soma-schema init --rules claude    # appends to CLAUDE.md
soma-schema init --rules cursor    # writes .cursor/rules/soma-schema.mdc
soma-schema init --rules all       # writes all supported files
soma-schema init --rules none      # skip agent rules entirely
```

### 2. Edit the setup SQL

Open `migrations/00_setup/01_schema.sql` and add your schema declaration. This file runs before every `up` and is never tracked — it must be idempotent:

```sql
CREATE SCHEMA IF NOT EXISTS myapp;
```

Files in `00_setup/` are never listed in `migration-order.yaml` and are never orphan-flagged.

### 3. Try the example migration

The scaffolded example migration is immediately runnable. Set your connection URL and apply:

```sh
export DATABASE_URL="postgres://user:pass@localhost/mydb"
soma-schema --database-url "$DATABASE_URL" --migrations migrations/ up
```

`DATABASE_URL` can also be read from the environment; `--database-url` overrides it.

### 4. Write your own first migration

Add a file under `migrations/01_migrated/1/`:

```sql
-- migrations/01_migrated/1/20260101_01_init.sql

CREATE TABLE myapp.users (
    id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL
);

-- DOWN ==
DROP TABLE IF EXISTS myapp.users;
```

The delimiter must be exactly `-- DOWN ==` (trimmed). Everything before it is the UP section; everything after is DOWN.

### 5. Register it in the manifest

Edit `migrations/migration-order.yaml`:

```yaml
manifest_version: 1
versions:
  - version: 1
    description: "Initial schema"
    migrations:
      - file: "20260101_01_example.sql"
        created: "2026-01-01"
        author: "soma-schema"
        why: "Example migration"
      - file: "20260101_01_init.sql"
        created: "2026-01-01"
        author: "you"
        why: "Create users table"
```

Every `.sql` file under a version folder must have an entry here. A file on disk but missing from the manifest causes an `OrphanMigration` error.

### 6. Apply

```sh
soma-schema --database-url "$DATABASE_URL" --migrations migrations/ up
```

### 7. Check status

```sh
soma-schema --database-url "$DATABASE_URL" --migrations migrations/ status
```

Output shows applied migrations (with applied-at timestamp and batch number) and pending migrations.

### 8. Open the explorer

```sh
soma-schema --migrations migrations/ explorer
```

No database connection needed. Opens a self-contained HTML page with a schema ERD, migration timeline, and seed-data tables.

### 9. Roll back

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
- [Use with AI](@/use-with-ai.md) — how the agent-rules contract works
