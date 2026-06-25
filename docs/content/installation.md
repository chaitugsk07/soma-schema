+++
title = "Installation"
description = "How to install soma-schema as a CLI binary or add it as a library crate."
weight = 10
+++

## One-command setup

The fastest way to get started is:

```sh
cargo install soma-schema && soma-schema init
```

This installs the CLI from crates.io and immediately scaffolds your project — see [What `init` creates](#what-init-creates) below.

## As a CLI binary only

```sh
cargo install soma-schema
```

This builds the binary from crates.io and places it in `~/.cargo/bin/`. The binary requires the `cli` feature, which is on by default.

Once installed:

```sh
soma-schema --help
```

## As a library crate

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
soma-schema = "0.3"
```

Or use `cargo add`:

```sh
cargo add soma-schema
```

### Disabling the CLI feature

The `cli` feature pulls in `clap`. If you are using soma-schema purely as an embedded library, disable it:

```toml
soma-schema = { version = "0.3", default-features = false }
```

Most services that run migrations at startup should use `default-features = false`.

### Dependency forms for soma services

```toml
# Active development (sibling clone) — library only, no CLI:
soma-schema = { path = "../soma-schema", default-features = false }

# Pinned to a git tag:
soma-schema = { git = "https://github.com/chaitugsk07/soma-schema", tag = "v0.3.0", default-features = false }

# From crates.io once published:
soma-schema = { version = "0.3", default-features = false }
```

## What `init` creates

`soma-schema init [DIR]` scaffolds everything in one step:

```text
.                          ← repo root
├── AGENTS.md              ← agent-rules file (written by default)
└── migrations/            ← DIR, defaults to "migrations"
    ├── migration-order.yaml
    ├── 00_setup/
    │   └── 01_schema.sql  ← idempotent bootstrap (CREATE SCHEMA IF NOT EXISTS)
    └── 01_migrated/
        └── 1/
            └── 20260101_01_example.sql  ← runnable CREATE TABLE + DROP DOWN
```

The example migration is already listed in `migration-order.yaml`, so `soma-schema up` works immediately after `init`.

### `init` flags

| Flag | Default | Description |
| --- | --- | --- |
| `[DIR]` | `migrations` | Where to scaffold the migrations directory |
| `--rules <agents\|claude\|cursor\|windsurf\|all\|none>` | `agents` | Which agent-rules file(s) to write or append to |
| `--skill` | off | Also install the `/soma-schema` Claude skill to `~/.claude/skills/soma-schema/SKILL.md` |
| `--explore` | off | Open the visual migration explorer after scaffolding |

If the target rules file already exists, the soma-schema section is appended idempotently — existing content is never overwritten.

## Requirements

- **Rust:** stable toolchain (1.75+ recommended)
- **Database:** PostgreSQL — MySQL, SQLite, and other backends are [planned](@/roadmap.md) but not yet implemented
- **Pool:** when using the library, your `PgPool` must allow `max_connections >= 2` (one connection is held for the run-scoped advisory lock)
