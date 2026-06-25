# Changelog

All notable changes to soma-schema are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Explorer feature** (`soma-schema explorer`): visual schema ERD + seed-data viewer.
  Parses every UP migration to build an entity-relationship diagram (tables, columns,
  foreign keys, primary keys, nullability) and a seed-data tab showing inserted rows.
  Outputs a self-contained HTML file — no server, no database needed.
- **`status` command**: `soma-schema status` prints applied, pending, and drift errors
  without modifying the database.
- **Checksum drift surfaced in `status`**: `MigrationStatus.drift_errors` collects all
  integrity violations (checksum mismatches and applied-but-missing files) in a single
  call. `status()` returns `Ok` even when drift is present so it can be used as a
  pre-flight check without aborting.
- **Public re-exports** at the crate root: `discover`, `AppliedMigration`, `LockGuard`,
  `MigrationDriver`, `Migration`, `SetupFile`, `MigrationStatus`, `PendingMigration`,
  `PostgresConfig`, `PostgresDriver`, `build_json`, `render_html`.
- **`#[non_exhaustive]`** on `Error`, `AppliedMigration`, `PendingMigration`, and
  `MigrationStatus` — callers must use wildcard match arms and field access rather than
  struct construction, keeping the API forward-compatible.
- **MSRV pin**: `rust-version = "1.75"` in `Cargo.toml`; checked in CI.
- **CI hardening**: MSRV job (Rust 1.75), cargo-deny job (advisories + licenses),
  in-memory driver tests (no Postgres), explorer tests (no Postgres), doctests.

### Fixed
- **XSS in generated HTML**: `</script>` sequences inside embedded JSON are now escaped
  to `<\/script>` so data values cannot break out of the enclosing `<script>` block.
- **Accessibility**: explorer HTML ships with ARIA landmark roles, focus-visible outlines,
  and sufficient colour contrast, meeting WCAG 2.1 AA targets.

## [0.1.0] — 2026-01-01

Initial release.

- Plain SQL files with `UP` / `-- DOWN ==` / `DOWN` sections.
- `migration-order.yaml` manifest defines apply order and rollback order (exact reverse).
- SHA-256 checksum on full file content; drift detected on every `up()` and `down()` call.
- PostgreSQL advisory lock held for the duration of each `up()` / `down()` call.
- Apply + tracking-row insert in a single transaction per migration.
- `soma-schema` CLI: `up`, `down --steps N`, `status`, `init`.
- Throwaway schema isolation in integration tests (`_sdm_test_<uuid>`).

[Unreleased]: https://github.com/chaitugsk07/soma-schema/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/chaitugsk07/soma-schema/releases/tag/v0.1.0
