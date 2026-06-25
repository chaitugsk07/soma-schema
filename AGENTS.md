# Agent rules — soma-schema migrations

This file is the canonical source of rules for AI coding agents working with
soma-schema migrations in any repo. It is auto-read by Claude Code, OpenAI Codex,
Cursor, Zed, Google Antigravity, and most other agentic tools.

---

## Database migrations — soma-schema

This project uses soma-schema for all database migrations.
Docs: https://chaitugsk07.github.io/soma-schema
Repo: https://github.com/chaitugsk07/soma-schema

When asked to change the database schema, generate a soma-schema migration. Do NOT
hand-edit the database or any already-applied migration file.

### How migrations work

- Plain SQL files under `migrations/01_migrated/<version>/`, each with an UP section,
  a line that trims to exactly `-- DOWN ==`, then a DOWN section.
- Every file is listed in `migrations/migration-order.yaml`, which defines apply order
  (top to bottom) and rollback order (the exact reverse).
- The runner checksums the whole file (UP+DOWN) and applies + records each migration
  in one transaction, under a database advisory lock.

### Rules when generating a migration

1. **NEVER edit a migration that was already applied** — checksum drift errors the next
   run. To change deployed schema, write a NEW migration.
2. Add every new `.sql` to `migration-order.yaml` in the correct version block, in
   apply order.
3. Write a DOWN that undoes UP in FK-safe reverse order (drop children before parents).
4. Seeds are idempotent: UP uses `ON CONFLICT DO NOTHING` so re-runs are safe.
5. One schema per service; `00_setup/` must `CREATE SCHEMA IF NOT EXISTS` it (idempotent
   only — no tracked state in setup files).
6. Follow this project's existing SQL conventions (naming, types, allowed extensions).

### To add a migration

```
1. Create  migrations/01_migrated/<version>/<YYYYMMDD>_<NN>_<name>.sql
           with UP section, then "-- DOWN ==" delimiter, then DOWN section.

2. Add it  to migrations/migration-order.yaml (file, created, author, why).

3. Verify  soma-schema --migrations migrations status   # confirms it is pending

4. Apply   soma-schema --migrations migrations up

5. Freeze  Never touch the file again once applied.
```

### SQL file format

```sql
-- UP section (everything before the delimiter)
CREATE TABLE myapp.things (
    id   uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    name text NOT NULL
);

-- DOWN ==
DROP TABLE IF EXISTS myapp.things;
```

### migration-order.yaml entry format

```yaml
manifest_version: 1
versions:
  - version: 1
    description: "Initial schema"
    migrations:
      - file: "20260101_01_init.sql"
        created: "2026-01-01"
        author: "you"
        why: "Core tables"
```

### Key invariants — never break these

| Invariant | Why |
|-----------|-----|
| Never edit an applied file | Checksum drift aborts the next `up()` |
| Every `.sql` in a version folder must be in the manifest | OrphanMigration error |
| Every manifest entry must have a matching file | MissingFile error |
| Version folders sorted numerically (1, 2, 10 — not 1, 10, 2) | Manifest controls order |
| `00_setup/` files are idempotent and untracked | They run on every `up()` |
| Advisory lock key must be unique per service in a shared database | Prevents cross-service lock conflicts |
| Pool must allow ≥ 2 connections | One is held for the advisory lock |
| Apply + tracking row in one transaction | No split state on crash |
