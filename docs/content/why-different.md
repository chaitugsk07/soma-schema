+++
title = "Why soma-schema is different"
description = "How soma-schema compares to Flyway, Liquibase, sqlx-migrate, and Alembic — and why the differences matter for AI-assisted development."
weight = 5
+++

Flyway, Liquibase, sqlx-migrate, Alembic — all built for humans typing commands. soma-schema is built for a codebase where an AI agent writes most of the migrations. That design constraint shaped every decision.

### AI-native by default

soma-schema ships agent rules and a Claude skill so an AI coding agent generates correct migrations from the start — proper UP/DOWN sections, manifest entries, FK-safe order — rather than guessing a convention and getting corrected. The rules are small enough to paste into a `CLAUDE.md`; no boilerplate required on the human's side.

### Explicit order, not filenames

`migration-order.yaml` defines the apply sequence and, as its exact reverse, the rollback sequence. There are no naming tricks, no timestamp races, no lexicographic edge cases. When a table has foreign keys into three others, you control which migration runs first and which rolls back last — the manifest says so, plainly. Tools that sort by filename break silently when naming drifts.

### Whole-file drift detection

The SHA-256 checksum covers the entire file — UP and DOWN together. If anyone edits the DOWN section of a migration that has already been applied, the next `up`, `down`, or `status` stops with a `ChecksumDrift` error. Most tools hash only the UP section; some hash metadata strings; a few do nothing at all.

### Crash- and concurrency-safe

The migration SQL and the tracking-table row commit in a single transaction. A crash between apply and record produces nothing — no half-applied migration, no phantom entry. On top of that, a single Postgres advisory lock is held from the first operation through the last, so concurrent runners block each other rather than collide.

### Library and CLI, one tool

Embed `Migrator::from_root("migrations").up(&driver).await?` at service startup, or call `soma-schema up` in a deploy pipeline — same logic, same manifest, same safety guarantees either way. No JVM, no daemon, no sidecar process.

### One tool, many databases

The `MigrationDriver` trait is the only abstraction point: six async methods. Postgres is stable today. MySQL, SQLite, CockroachDB, SurrealDB, and MongoDB are [on the roadmap](@/roadmap.md). The manifest, checksum, and locking logic are all above the driver and need no changes when a new backend lands.

---

For a detailed side-by-side comparison with sqlx-migrate, refinery, diesel_migrations, Flyway, and Alembic, see [competitor-analysis.md](https://github.com/chaitugsk07/soma-schema/blob/main/docs/competitor-analysis.md) in the repository.
