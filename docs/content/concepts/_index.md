+++
title = "Concepts"
description = "How soma-schema's four core mechanisms work under the hood."
weight = 25
sort_by = "weight"
+++

These pages explain the four mechanisms that distinguish soma-schema from filename-ordered tools. You do not need to read them to use the tool, but they answer the why behind the invariants.

- [Manifest ordering](@/concepts/manifest-ordering.md) — how `migration-order.yaml` drives both apply and rollback order, and why filename sort is insufficient for FK-safe rollback.
- [Checksum drift](@/concepts/checksum-drift.md) — what the SHA-256 checksum covers, when drift errors fire, and how to recover.
- [Advisory lock](@/concepts/advisory-lock.md) — how the run-scoped Postgres advisory lock works and why it is held for the full call rather than per-migration.
- [Tracking table](@/concepts/tracking-table.md) — the schema of `00_schema_migrations` and what each column records.
