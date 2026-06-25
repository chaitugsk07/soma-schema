+++
title = "Installation"
description = "How to install soma-schema as a CLI binary or add it as a library crate."
weight = 10
+++

## As a CLI binary

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
soma-schema = "0.2"
```

Or use `cargo add`:

```sh
cargo add soma-schema
```

### Disabling the CLI feature

The `cli` feature pulls in `clap`. If you are using soma-schema purely as an embedded library, disable it:

```toml
soma-schema = { version = "0.2", default-features = false }
```

Most services that run migrations at startup should use `default-features = false`.

### Dependency forms for soma services

```toml
# Active development (sibling clone) — library only, no CLI:
soma-schema = { path = "../soma-schema", default-features = false }

# Pinned to a git tag:
soma-schema = { git = "https://github.com/chaitugsk07/soma-schema", tag = "v0.2.0", default-features = false }

# From crates.io once published:
soma-schema = { version = "0.2", default-features = false }
```

## Requirements

- **Rust:** stable toolchain (1.75+ recommended)
- **Database:** PostgreSQL — MySQL, SQLite, and other backends are [planned](@/roadmap.md) but not yet implemented
- **Pool:** when using the library, your `PgPool` must allow `max_connections >= 2` (one connection is held for the run-scoped advisory lock)
