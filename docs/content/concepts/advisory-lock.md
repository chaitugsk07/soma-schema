+++
title = "Advisory lock"
description = "How the run-scoped Postgres advisory lock prevents concurrent migration runners from colliding."
weight = 30
+++

<div class="callout callout-info"><span class="callout-icon">ℹ</span><p>When multiple services share one Postgres database, each must use a <strong>distinct</strong> <code>advisory_lock_key</code>. The default key <code>918273645</code> is database-global — two services with the same key will serialize their migrations against each other even across different schemas.</p></div>

## The problem

Without a locking mechanism, two `up` processes running simultaneously — a deploy pipeline and a service restart, for example — can both read the tracking table, both decide the same migration is pending, and both attempt to apply it. Depending on the migration, the result is either a failed transaction (if the DDL is not idempotent) or duplicate data (if it is).

## How soma-schema locks

At the start of every `up`, `down`, or `status` call, soma-schema acquires a Postgres session-level advisory lock using `pg_advisory_lock`. The lock is held on a dedicated connection for the entire duration of the call. When the call returns — or if the call panics — a RAII guard (`PgLockGuard`) releases the lock via its `Drop` implementation.

A second runner that arrives while the lock is held will block on `pg_advisory_lock` until the first runner finishes. It then acquires the lock itself, reads the updated tracking table, and proceeds with whatever remains.

## Per-run, not per-migration

Some tools acquire a lock before each individual migration and release it after. This leaves a window between migrations where another runner can observe a partially-applied batch and start applying the same remaining migrations.

soma-schema holds the lock for the entire `up` or `down` call. From the perspective of any other runner, a batch of migrations is either not started or fully complete — there is no observable in-between state.

## The lock key

The lock is keyed by an `i64` advisory lock key. The default is `918273645`. When multiple services share one Postgres database, each must use a distinct key — Postgres advisory locks are database-global, so two services using the same key would serialize their `up` calls against each other even if they touch different schemas.

Set a unique key per service in `PostgresConfig`:

```rust
PostgresConfig {
    advisory_lock_key: 0x_50A_1A33, // unique to this service
    ..Default::default()
}
```

Pick a constant, leave a comment naming the service that owns it.

## Two connections

`PostgresDriver` always uses two connections:

- One connection holds the advisory lock for the duration of the call.
- One connection does the actual migration work.

This is why `PgPool` must be configured with `max_connections >= 2`. The `PostgresDriver::new` constructor returns `PoolTooSmall` if this requirement is not met.

The separation is necessary because Postgres session-level advisory locks are tied to the connection that acquired them. If the lock connection and the work connection were the same, a transaction rollback on the work side could release the lock prematurely.
