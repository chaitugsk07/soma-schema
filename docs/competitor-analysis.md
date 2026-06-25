# soma-schema — Competitor Analysis

## Positioning

soma-schema fills the gap between two unsatisfying extremes in the Rust/database migration space.

On one side: the lightweight Rust-native tools — refinery, diesel_migrations, sqlx-migrate. They order migrations by filename sort, carry weak or no checksums, and have no locking. They work fine for simple cases but fail quietly when you need deterministic rollback order across FK-linked tables, or when you need to catch someone editing a deployed file.

On the other side: the JVM tools — Flyway and Liquibase. They have the features (full-file checksums, advisory locking, atomic apply+track), but they require a JVM, parts are commercial, and they are not usable as a Rust library.

soma-schema's closest peer on features is **Flyway Community**: both do full-file checksums, both hold an advisory lock for the full migration run, both apply and track in one atomic transaction. soma-schema's advantage is a native Rust binary/library, no JVM, and an explicit YAML manifest that makes rollback order deterministic by construction rather than by naming convention.

soma-schema is the only Rust tool combining:
- Full-file drift detection (UP and DOWN together)
- Manifest-defined, FK-safe rollback order
- Run-scoped advisory locking (not per-migration)
- Dual delivery as a library crate and a standalone CLI

---

## Full comparison table

Star counts are approximate as of mid-2026.

| Tool | Lang | Format | Ordering | Checksum / Drift | Locking | Lib+CLI | License | ~Stars |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **soma-schema** | **Rust** | **Plain SQL (1 file UP+DOWN)** | **YAML manifest** | **Full-file (UP+DOWN)** | **Advisory, full-run** | **Both** | **Apache-2.0** | **new** |
| sqlx migrate | Rust | Plain SQL | Filename lexical | UP-only hash | Advisory (Pg) | Both | MIT/Apache | ~14k |
| refinery | Rust | Plain SQL | Filename lexical | Metadata hash | None | Lib only | MIT | ~1.1k |
| diesel_migrations | Rust | Plain SQL | Filename sort | None | None | Lib (ORM-tied) | MIT/Apache | ~13k |
| dbmate | Go | Plain SQL | Filename sort | None | Advisory | CLI only | MIT | ~5.8k |
| golang-migrate | Go | Plain SQL | Filename sort | None (dirty flag) | Various | Both | MIT | ~16k |
| goose | Go | SQL or Go funcs | Filename sort | None | Advisory | Both | MIT | ~8k |
| Atlas | Go | HCL/SQL DSL | State-based diff | Schema hash | Advisory | Both | Apache/BSL | ~6k |
| Flyway | JVM | Plain SQL | Version prefix | Full-file | Advisory | Both | Apache + commercial | ~8k |
| Liquibase | JVM | XML/YAML/SQL DSL | Changelog order | Full-file | Advisory | Both | Apache + commercial | ~5k |
| sqitch | Perl | Plain SQL | Dependency graph | Verify scripts | None native | CLI only | MIT | ~2.8k |
| Alembic | Python | Python DSL | Revision chain | Revision hash | None native | Lib+CLI | MIT | ~10k |

---

## Per-tool notes

### sqlx migrate

sqlx's built-in migrator is a natural starting point for Rust projects already using sqlx. It orders by filename, hashes only the UP section, and holds an advisory lock per-migration rather than for the full run. If you edit a deployed DOWN section, the drift is invisible until you try to roll back. soma-schema checksums the entire file and holds the lock for the whole `up`/`down` call.

### refinery

Lightweight, no ORM dependency, library-only. Filename-based ordering; checksum covers a metadata string (version + name), not file content, so editing SQL in a deployed file won't be caught. No advisory locking, so concurrent runners can collide. soma-schema adds locking, full-content checksums, and a CLI.

### diesel_migrations

Tightly coupled to the Diesel ORM. Migrations are compiled into the binary, which means no runtime drift detection — there is no tracking table checksum at all. Ordering is by filename. If you are already in the Diesel ecosystem this is convenient; if you are not, it is a significant dependency to pull in for migrations alone.

### dbmate

A Go CLI with no library API. Ordered by filename, no checksum/drift detection. Has advisory locking. Useful as a language-agnostic CLI tool; soma-schema adds checksums and is usable as a Rust library.

### golang-migrate

Very widely used Go library+CLI. Supports many database backends. Ordering by filename, no per-file checksums (uses a boolean "dirty" flag instead). Advisory locking varies by driver. soma-schema's advantage is full-file drift detection and explicit manifest ordering; golang-migrate's advantage is its driver breadth.

### goose

Go library+CLI with support for SQL migrations and Go function migrations. Filename-sorted, no checksums. Has advisory locking. Go function migrations are a useful feature soma-schema does not offer.

### Atlas

Takes a different philosophy: schema-as-desired-state, auto-generates migration SQL by diffing current schema against target. Useful when you want generated migrations rather than hand-written ones. If you need to hand-author SQL (complex data migrations, custom index strategies, partitioning) Atlas is less comfortable. soma-schema does not generate SQL; it runs what you write.

### Flyway

The most feature-complete migration tool. Full-file checksums, advisory locking, atomic apply+tracking, undo migrations (commercial), repair commands. Requires a JVM. Community edition is Apache-2.0; enterprise features are commercial. soma-schema covers the core feature set without the JVM, as a Rust crate or binary.

### Liquibase

Similar scope to Flyway but uses XML/YAML/JSON changelog format rather than plain SQL. Supports many databases. Strong enterprise feature set, partly commercial. Significantly more complex to set up than plain-SQL tools. soma-schema is plain SQL only.

### sqitch

Unique dependency-graph ordering — migrations declare what they depend on, and sqitch derives apply/revert order from that graph. No built-in drift detection; relies on verify scripts you provide. Perl, CLI-only. soma-schema uses explicit YAML order rather than dependency inference.

### Alembic

Python library+CLI, tightly coupled to SQLAlchemy. Revision-chain ordering (each migration names its parent). Can auto-generate migrations from SQLAlchemy model diffs. Hashes revision IDs, not file content. No advisory locking by default. If you are in a Python/SQLAlchemy stack it is the natural choice; soma-schema does not try to serve that use case.

---

## When NOT to choose soma-schema

- **You want auto-generated SQL from a schema diff.** Use Atlas. soma-schema only runs SQL you write.

- **You are already all-in on Diesel ORM.** diesel_migrations has zero extra dependencies and compiles your migrations into the binary. soma-schema requires a runtime migrations directory.

- **You need MySQL or SQLite today.** soma-schema is Postgres-only at this point. MySQL and SQLite drivers are on the roadmap but do not exist yet.

- **You need a language-agnostic CLI for a non-Rust stack.** dbmate or golang-migrate are more established for that use case. soma-schema can be used this way (it is a standalone binary) but has fewer database backends.

- **You need undo/repair tooling and enterprise support.** Flyway Enterprise covers those. soma-schema is a community open-source project.
