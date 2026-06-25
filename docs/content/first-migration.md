+++
title = "Your first migration"
description = "Step-by-step: scaffold → write UP/DOWN → add to manifest → apply → check status → roll back."
weight = 40
+++

This tutorial walks through writing and running a migration from scratch. It assumes you have soma-schema installed and a Postgres database available.

## Step 1: Scaffold

```sh
soma-schema init migrations/
```

You get this layout:

```text
migrations/
  migration-order.yaml
  00_setup/
    01_schema.sql
  01_migrated/
    1/
```

## Step 2: Set up your schema

Open `migrations/00_setup/01_schema.sql` and write the CREATE SCHEMA statement. This file runs on every `up` call and must be idempotent:

```sql
CREATE SCHEMA IF NOT EXISTS myapp;
```

You can add `CREATE EXTENSION` calls here too, if needed.

## Step 3: Write the migration file

Create the file `migrations/01_migrated/1/20260101_01_users.sql`:

```sql
CREATE TABLE myapp.users (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    email      TEXT        NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- DOWN ==
DROP TABLE IF EXISTS myapp.users;
```

The line `-- DOWN ==` is the separator. Everything above it runs on `up`; everything below runs on `down`.

## Step 4: Add it to the manifest

Open `migrations/migration-order.yaml` and add your migration entry:

```yaml
manifest_version: 1
versions:
  - version: 1
    description: "Initial schema"
    migrations:
      - file: "20260101_01_users.sql"
        created: "2026-01-01"
        author: "you"
        why: "Create users table"
```

Every `.sql` file under a version folder must be listed here. The manifest is the single source of truth for apply order — soma-schema does not fall back to filename sort.

## Step 5: Apply

```sh
soma-schema --database-url "$DATABASE_URL" --migrations migrations/ up
```

If the database and pool are reachable, soma-schema will:

1. Acquire a run-scoped advisory lock.
2. Run the setup SQL (`00_setup/01_schema.sql`).
3. Check the tracking table (`00_schema_migrations`) for already-applied migrations.
4. Compare checksums of applied files — any drift is an error before anything runs.
5. Apply `20260101_01_users.sql` and record it in the tracking table in a single transaction.
6. Release the lock.

## Step 6: Check status

```sh
soma-schema --database-url "$DATABASE_URL" --migrations migrations/ status
```

Output lists applied migrations (filename, batch, applied-at, who) and pending migrations. After step 5 you should see `20260101_01_users.sql` as applied and no pending migrations.

## Step 7: Roll back

```sh
soma-schema --database-url "$DATABASE_URL" --migrations migrations/ down
```

This runs the DOWN section of the last applied migration (`DROP TABLE IF EXISTS myapp.users;`) and removes its row from the tracking table — in one transaction. After this, `status` shows it as pending again.

## What not to do

**Do not edit the file after applying it.** The checksum is over the full file content. Even changing a comment triggers `ChecksumDrift` on the next run. If you need to change the schema, write a new migration.

**Do not skip the manifest entry.** A `.sql` file with no manifest entry is an `OrphanMigration` error.

## Adding a second migration

Add `migrations/01_migrated/1/20260102_01_add_role.sql` (same version folder, next file in sequence):

```sql
ALTER TABLE myapp.users ADD COLUMN role TEXT NOT NULL DEFAULT 'member';

-- DOWN ==
ALTER TABLE myapp.users DROP COLUMN role;
```

Add it to the manifest, after the first entry in version 1:

```yaml
      - file: "20260102_01_add_role.sql"
        created: "2026-01-02"
        author: "you"
        why: "Add role column"
```

Run `up` again. Only the new migration runs — already-applied migrations are skipped.
