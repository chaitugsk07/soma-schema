+++
title = "Use with AI agents"
description = "Drop-in rules so any agentic coding tool follows the soma-schema migration conventions, with runner guardrails that catch what a non-deterministic agent gets wrong."
weight = 15
+++

soma-schema is AI-native. It works with any agentic coding tool — Claude Code, OpenAI Codex, Cursor, Windsurf, GitHub Copilot, Google Antigravity, Aider, and more. It ships ready-to-paste agent rules that teach the agent exactly how migrations work here — correct UP/DOWN sections, the manifest entry, FK-safe order — and, critically, that it must never edit a file that has already been applied. Two minutes of setup means the agent follows the conventions from the first try, and soma-schema's checks (checksum drift, ordering, never editing an applied file) catch anything a non-deterministic agent still gets wrong.

## Step 1 — add the rules to your agent's rules file

Drop the block below into your repo's agent rules file (see the table below for which file your tool reads). Any agent working in that repo will follow these rules automatically whenever it touches migrations.

```text
## Database migrations — soma-schema

This project uses soma-schema for all database migrations.
Docs: https://chaitugsk07.github.io/soma-schema · Repo: https://github.com/chaitugsk07/soma-schema

When asked to change the database schema, generate a soma-schema migration. Do NOT
hand-edit the database or any already-applied migration file.

How migrations work here:
- Plain SQL files under migrations/01_migrated/<version>/, each with an UP section,
  a line that trims to exactly "-- DOWN ==", then a DOWN section.
- Every file is listed in migrations/migration-order.yaml, which defines apply order
  (top to bottom) and rollback order (the exact reverse).
- The runner checksums the whole file (UP+DOWN) and applies + records each migration
  in one transaction, under a database advisory lock.

Rules when generating a migration:
1. NEVER edit a migration that was already applied — checksum drift errors the next run.
   To change deployed schema, write a NEW migration.
2. Add every new .sql to migration-order.yaml in the correct version block, in apply order.
3. Write a DOWN that undoes UP in FK-safe reverse order (drop children before parents).
4. Seeds are idempotent: UP uses ON CONFLICT DO NOTHING so re-runs are safe.
5. One schema per service; 00_setup/ must CREATE SCHEMA IF NOT EXISTS it (idempotent only).
6. Follow this project's existing SQL conventions (naming, types, allowed extensions).

To add a migration:
- Create migrations/01_migrated/<version>/<YYYYMMDD>_<NN>_<name>.sql with UP + "-- DOWN ==" + DOWN.
- Add it to migration-order.yaml (created/author/why).
- Run: soma-schema --migrations migrations status   (confirm it's pending)
- Run: soma-schema --migrations migrations up        (apply it)
- Never touch the file again once applied.
```

### Where each tool reads its rules

| File | Tool(s) |
| ---- | ------- |
| `AGENTS.md` | Cross-tool standard — OpenAI Codex, Cursor, Zed, Google Antigravity, most agentic tools |
| `CLAUDE.md` | Claude Code |
| `.cursor/rules/*.mdc` | Cursor (legacy `.cursorrules`) |
| `.windsurf/rules/` | Windsurf (legacy `.windsurfrules`) |
| `.github/copilot-instructions.md` | GitHub Copilot |
| `CONVENTIONS.md` | Aider |

Keep `AGENTS.md` as the source of truth. For a tool that reads a different file, copy the same rules there or have that file say "Follow AGENTS.md." On any OS — these are plain text files in your repo.

## Claude Code bonus — the /soma-schema skill

Claude Code can also load these rules as a reusable slash-command skill (other tools use the rules file above). A Claude Code skill wires up a `/soma-schema` slash command and auto-routes on any migration work in any repo. Create the file at `~/.claude/skills/soma-schema/SKILL.md` with the content below, then `/soma-schema` is available in every project.

```markdown
---
name: soma-schema
description: >
  How to adopt and operate the soma-schema migration tool (Rust, Postgres) correctly
  in any repo: dependency setup, the migrations/ + migration-order.yaml contract,
  UP/DOWN files, run-at-startup vs CLI, and the non-negotiable invariants (never edit
  applied files, manifest-complete ordering, FK-safe DOWN, one schema + unique advisory
  lock key per service, pool >= 2). Invoke with /soma-schema.
metadata:
  version: "1.0.0"
---

# soma-schema — Migration Runner Rules

Docs: https://chaitugsk07.github.io/soma-schema
Repo: https://github.com/chaitugsk07/soma-schema

Rules for wiring and operating soma-schema in any project. Covers the runner contract:
dependency, directory structure, library/CLI API, and invariants. SQL content rules
(naming, types, allowed extensions) follow your project's own conventions.

---

## When to apply

- Designing or wiring soma-schema into a new service.
- Writing or reviewing any migration file (.sql) or migration-order.yaml.
- Debugging ChecksumDrift, OrphanMigration, MissingFile, or PoolTooSmall errors.
- Any up/down/status workflow question.

---

## Cargo dependency

The cli feature is on by default and pulls in clap. Services embedding the library
should disable it:

    # Sibling clone (active dev):
    soma-schema = { path = "../soma-schema", default-features = false }

    # Pinned git tag:
    soma-schema = { git = "https://github.com/chaitugsk07/soma-schema", tag = "v0.2.0", default-features = false }

    # crates.io (once published):
    soma-schema = { version = "0.2", default-features = false }

Keep default-features = true only if you also need the soma-schema CLI binary built from
this dep. Embedding the library at startup is the norm; always set default-features = false
in that case.

---

## migrations/ directory contract

    migrations/
      migration-order.yaml      # authoritative manifest — apply AND rollback order
      00_setup/                 # idempotent bootstrap; runs every up(), never tracked
        01_schema.sql
      01_migrated/
        1/                      # integer version folder, sorted numerically
          20260101_01_init.sql
      02_inprogress/            # optional staging area for in-flight work

### SQL file format

    -- UP section
    CREATE TABLE myapp.roles (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), name TEXT NOT NULL);

    -- DOWN ==
    DROP TABLE IF EXISTS myapp.roles;

The delimiter is the line that trims to exactly "-- DOWN ==". Checksum covers the entire
file — UP and DOWN together. Any post-deploy edit triggers ChecksumDrift.

### migration-order.yaml

    manifest_version: 1
    versions:
      - version: 1
        description: "Initial schema"
        migrations:
          - file: "20260101_01_init.sql"
            created: "2026-01-01"
            author: "you"
            why: "Core tables"

Rules:
- manifest_version must be 1.
- file is a bare filename — no path separators, must end in .sql.
- File on disk, absent from manifest → OrphanMigration.
- Manifest entry, no file → MissingFile.
- Version folders sorted numerically. 00_setup/ files never listed here.

---

## Library integration (run at startup)

    use soma_schema::{Migrator, PostgresConfig, PostgresDriver};

    let driver = PostgresDriver::new(pool.clone(), PostgresConfig {
        schema: Some("myapp".into()),
        advisory_lock_key: 0x_50A_1A33, // unique per service
        ..Default::default()             // table: "00_schema_migrations"
    })?;
    Migrator::from_root("migrations").up(&driver).await?;

Pool must allow >= 2 connections (one held for the advisory lock).

---

## CLI usage

    soma-schema --database-url "$DATABASE_URL" --schema myapp --migrations migrations up
    soma-schema --database-url "$DATABASE_URL" --schema myapp --migrations migrations status
    soma-schema --database-url "$DATABASE_URL" --schema myapp --migrations migrations down --steps 1

---

## Non-negotiable invariants

1. NEVER edit an applied migration file. Any change — even a comment — triggers
   ChecksumDrift. Write a new migration instead.

2. Every new .sql must be listed in migration-order.yaml in the correct version block,
   in apply order.

3. Write a DOWN section for every migration unless genuinely irreversible. DOWN must undo
   UP in FK-safe reverse order. Rollback order = reverse of manifest position, so manifest
   order must be FK-correct going forward.

4. Seeds are idempotent migrations. UP uses ON CONFLICT DO NOTHING so re-running after
   rollback is safe.

5. One schema per service. Set PostgresConfig.schema; the tracking table lives in that
   schema. 00_setup/ must CREATE SCHEMA IF NOT EXISTS it.

6. Pool needs max_connections >= 2. One connection holds the advisory lock for the full
   up/down call. PostgresDriver::new returns PoolTooSmall if the pool cannot provide two.

7. Unique advisory_lock_key per service when services share one Postgres database.
   Advisory locks are database-global by key. Default is 918273645; pick a distinct i64
   constant per service and document it.

8. 00_setup/ SQL must be idempotent. CREATE SCHEMA IF NOT EXISTS, CREATE OR REPLACE
   FUNCTION, etc. Runs on every up(), never tracked.

9. SQL content follows your project's SQL conventions — naming, types, allowed
   extensions. soma-schema is the runner; it does not prescribe what the SQL may contain.

---

## Adding a migration — the agent loop

1. Pick or create the correct version folder under 01_migrated/ (or 02_inprogress/).
2. Create <YYYYMMDD>_<NN>_<name>.sql with UP + "-- DOWN ==" + DOWN.
3. Add an entry to migration-order.yaml with created, author, and why.
4. Run status to confirm the file appears as pending.
5. Run up (or start the service) to apply it.
6. Never touch the file again once it has been applied to any environment.
```

## What the agent does

Here is what happens when you tell the agent: *"Add an audit_logs table referencing organizations and users."*

The agent creates `migrations/01_migrated/2/20260625_01_audit_logs.sql`:

```sql
CREATE TABLE myapp.audit_logs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES myapp.organizations(id),
    user_id         UUID NOT NULL REFERENCES myapp.users(id),
    action          TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- DOWN ==
DROP TABLE IF EXISTS myapp.audit_logs;
```

Then it adds the manifest entry to `migrations/migration-order.yaml`:

```yaml
      - file: "20260625_01_audit_logs.sql"
        created: "2026-06-25"
        author: "agent"
        why: "Audit log table referencing organizations and users"
```

Then it runs:

```sh
soma-schema --migrations migrations status   # confirms the file is pending
soma-schema --migrations migrations up       # applies it
```

Notice the DOWN section drops `audit_logs` — not `organizations` or `users`. The agent knows rollback goes in reverse manifest order, so child tables come first.

---

For the full integration guide (Cargo dependency forms, library API, pool config), see [Consuming in your project](@/consuming-in-your-project.md) or the [`CONSUMING.md`](https://github.com/chaitugsk07/soma-schema/blob/main/CONSUMING.md) in the repository.
