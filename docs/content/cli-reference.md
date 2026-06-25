+++
title = "CLI reference"
description = "All soma-schema subcommands and flags."
weight = 60
+++

## Global flags

These flags apply to every subcommand that connects to a database.

| Flag | Env var | Default | Description |
| --- | --- | --- | --- |
| `--database-url <URL>` | `DATABASE_URL` | — | Postgres connection URL |
| `--migrations <PATH>` | — | `migrations` | Path to the migrations root directory |
| `--schema <SCHEMA>` | — | (connection default) | Target schema for the tracking table |
| `--table <TABLE>` | — | `00_schema_migrations` | Tracking table name |

`--database-url` overrides `DATABASE_URL` if both are set. The URL format is the standard libpq form: `postgres://user:password@host:5432/dbname`.

## `init`

Scaffold a new migrations directory and wire up agent rules in one step.

```sh
soma-schema init [DIR] [--rules <FORMAT>] [--skill] [--explore]
```

`DIR` defaults to `migrations`. Creates:

```text
.                          <- current directory (repo root)
+-- AGENTS.md              <- agent-rules file (default; see --rules)
+-- DIR/
    +-- migration-order.yaml
    +-- 00_setup/
    |   +-- 01_schema.sql
    +-- 01_migrated/
        +-- 1/
            +-- <date>_01_example.sql  <- runnable example, listed in manifest
```

| Flag | Default | Description |
| --- | --- | --- |
| `[DIR]` | `migrations` | Path for the migrations directory |
| `--rules <FORMAT>` | `agents` | Agent-rules file to write: `agents` (AGENTS.md), `claude` (CLAUDE.md), `cursor` (.cursor/rules/soma-schema.mdc), `windsurf` (.windsurf/rules/soma-schema.md), `all`, or `none` |
| `--skill` | off | Install the `/soma-schema` Claude skill to `~/.claude/skills/soma-schema/SKILL.md` |
| `--explore` | off | Open the visual migration explorer after scaffolding |

If the target rules file already exists, the soma-schema section is appended idempotently (skipped if a soma-schema migrations section is already present). Existing files are never overwritten or truncated.

Safe to run on an existing directory — it will not overwrite migration files that already exist.

## `up`

Apply all pending migrations.

```sh
soma-schema --database-url <URL> --migrations <PATH> up
```

Workflow:

1. Acquires a run-scoped advisory lock.
2. Runs all files in `00_setup/` (idempotent bootstrap).
3. Reads the tracking table to find applied migrations.
4. Recomputes checksums for applied files; aborts with `ChecksumDrift` if any differ.
5. Applies each pending migration in manifest order and records it in the tracking table — both in the same transaction.
6. Releases the lock.

Migrations that fail mid-run leave the database at the last successfully committed migration. The failed migration is not recorded.

## `down`

Roll back applied migrations.

```sh
soma-schema --database-url <URL> --migrations <PATH> down [--steps N]
```

| Flag | Default | Description |
| --- | --- | --- |
| `--steps <N>` | `1` | Number of migrations to roll back |

Rollback order is the exact reverse of manifest position — not filename sort. This is what makes FK-safe rollback deterministic.

Each migration's DOWN SQL and the deletion of its tracking-table row are executed in a single transaction. If a DOWN fails, the transaction is rolled back and the migration remains recorded as applied.

## `status`

Show applied and pending migrations.

```sh
soma-schema --database-url <URL> --migrations <PATH> status
```

Output lists:

- Applied migrations: filename, version, batch number, applied-at timestamp, applied-by (database role), execution time.
- Pending migrations: filename, version, `why` from the manifest.

Checksums are verified during `status` — a drift error will surface here before you attempt `up` or `down`.

## `explorer`

Build a self-contained visual explorer for your migrations directory — no database connection needed.

```sh
soma-schema --migrations <PATH> explorer [--format html|json] [--out <PATH>] [--no-open]
```

| Flag | Default | Description |
| --- | --- | --- |
| `--format <FORMAT>` | `html` | Output format: `html` (self-contained page) or `json` (raw data) |
| `--out <PATH>` | temp file (html) / stdout (json) | Write output to this path instead of the default |
| `--no-open` | off | Skip opening the browser after writing the HTML file |

The HTML output includes a schema ERD, a version-grouped migration timeline, and seed-data tables. For `--format json`, the structured data is written to stdout (or the path given by `--out`) so other tools can consume it.

`--database-url` is not required for this subcommand.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Success |
| `1` | Error (connection failure, drift, orphan, missing file, etc.) |

## Environment variable summary

| Variable | Used by |
| --- | --- |
| `DATABASE_URL` | All database-connecting subcommands |
