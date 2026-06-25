+++
title = "soma-schema"
description = "Plain SQL database migrations for Rust — manifest-defined ordering, full-file drift detection, and run-scoped advisory locking."
+++

soma-schema is a Postgres migration tool that ships as both a Rust library crate and a standalone CLI binary. You write plain SQL files with UP and DOWN sections in a single file, maintain a `migration-order.yaml` manifest that defines apply and rollback order explicitly, and soma-schema handles everything else: advisory locking so concurrent runners can't collide, SHA-256 checksums over the entire file so any post-deploy edit is caught, and atomic apply+track so a crash can't leave a half-applied state.

No ORM dependency. No opinions about your application framework. Postgres today, [more backends planned](@/roadmap.md).

## Four things it does differently

**Manifest-defined order.** `migration-order.yaml` lists every migration explicitly. Rollback order is the exact reverse of manifest position — not filename sort. This makes FK-safe rollback deterministic without naming conventions. sqlx-migrate, refinery, and diesel_migrations all order by filename.

**Full-file checksum drift detection.** The SHA-256 checksum covers the entire file — UP and DOWN together. Editing the DOWN section of a deployed migration is caught as `ChecksumDrift` the next time any command runs. sqlx hashes UP-only; refinery hashes metadata strings; diesel has no drift detection.

**Apply and track in one transaction.** The migration SQL and its tracking-table row commit atomically. A crash between those two operations cannot produce a migration that ran but has no record, or a record for a migration that never ran.

**Run-scoped advisory lock.** A single Postgres advisory lock is acquired once at the start of `up`, `down`, or `status` and held via a RAII guard until the call returns — even on panic. Concurrent runners block rather than collide.

## Where to start

- **New to soma-schema?** Start with [Quickstart](@/quickstart.md) — the 60-second CLI flow.
- **Want to embed it in a service?** See [Library usage](@/library-usage.md) and [Consuming in your project](@/consuming-in-your-project.md).
- **Need the full command reference?** See [CLI reference](@/cli-reference.md).
- **Want to understand the mechanics?** The [Concepts](@/concepts/_index.md) section covers manifest ordering, checksum drift, the advisory lock, and the tracking table.
