+++
title = "Manifest ordering"
description = "How migration-order.yaml defines apply and rollback order, and why filename sort is not enough."
weight = 10
+++

<div class="callout callout-forward"><span class="callout-icon">↑</span><p>Rollback order is the exact reverse of the manifest list — not filename sort. This makes FK-safe rollback deterministic regardless of how files are named or when they were added.</p></div>

## The problem with filename ordering

Most migration tools sort files lexicographically by name to determine apply order. This works for apply — you give files names like `001_create_users.sql`, `002_create_posts.sql` — but it breaks for rollback.

If `002_create_posts.sql` adds a foreign key that references the table created in `001_create_users.sql`, rolling back in reverse filename order means running `002`'s DOWN first (dropping posts) and then `001`'s DOWN (dropping users). That is correct here. But it relies on the developer maintaining a naming convention that encodes the dependency graph. Add a migration out of order, rename a file, or make a numbering mistake and the rollback order becomes wrong — silently.

## How soma-schema does it

`migration-order.yaml` lists every migration explicitly in apply order. Rollback order is the **exact reverse of the manifest list position** — not a filename sort, not a dependency computation.

```yaml
manifest_version: 1
versions:
  - version: 1
    description: "Initial schema"
    migrations:
      - file: "20260101_01_users.sql"
        created: "2026-01-01"
        author: "alice"
        why: "Core users table"
      - file: "20260101_02_posts.sql"
        created: "2026-01-01"
        author: "alice"
        why: "Posts table — FK to users"
```

Apply order: `01_users.sql`, then `02_posts.sql`.
Rollback order: `02_posts.sql`, then `01_users.sql`.

You control the order. The tool follows it exactly.

## Manifest validation rules

- `manifest_version` must be `1`.
- `file` is a bare filename — no path separators, must end in `.sql`.
- A `.sql` file in any version folder that is not listed in the manifest → `OrphanMigration` error at the start of any command.
- A manifest entry with no corresponding `.sql` file on disk → `MissingFile` error.
- Version folders are sorted numerically: `2` comes before `10`.
- Files in `00_setup/` are never listed in the manifest and are never orphan-flagged.

## Batch numbering

Each `up` call that applies at least one migration increments the batch number. All migrations applied in the same `up` call share a batch number. Rolling back with `down --steps 1` removes the most recently applied migration, regardless of batch. The batch number is stored in the tracking table for auditing.

## Position map and `down`

Internally, `down` builds a reverse-index from manifest position to migration, then walks backward by the number of steps requested. The position map is derived from the manifest, not from the tracking table's `applied_at` timestamps. This means rollback order is determined at build time by the manifest, not at runtime by when things happened — which is exactly the property that makes it deterministic.
