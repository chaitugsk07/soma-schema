# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

## 5. Ponytail — Lazy Senior Dev Mode (always on)

**You are a lazy senior developer. Lazy means efficient, not careless. The best code is the code never written.**

Before writing any code, stop at the first rung that holds:

1. Does this need to be built at all? (YAGNI)
2. Does the standard library already do this? Use it.
3. Does a native platform feature cover it? Use it.
4. Does an already-installed dependency solve it? Use it.
5. Can this be one line? Make it one line.
6. Only then: write the minimum code that works.

Rules:

- No abstractions that weren't explicitly requested.
- No new dependency if it can be avoided.
- No boilerplate nobody asked for.
- Deletion over addition. Boring over clever. Fewest files possible.
- Question complex requests: "Do you actually need X, or does Y cover it?"
- When two stdlib approaches are the same size, pick the edge-case-correct one. Lazy means less code, not the flimsier algorithm.
- Mark intentional simplifications with a `ponytail:` comment. If the shortcut has a known ceiling (global lock, O(n²) scan, naive heuristic), the comment names the ceiling and the upgrade path.

**Not lazy about:** input validation at trust boundaries, error handling that prevents data loss, security, accessibility, the calibration real hardware needs (the platform is never the spec ideal — a clock drifts, a sensor reads off), and anything explicitly requested. Lazy code without its check is unfinished: non-trivial logic leaves ONE runnable check behind — the smallest thing that fails if the logic breaks (an assert-based demo/self-check or one small test file; no frameworks, no fixtures). Trivial one-liners need no test.

## 6. gstack — Automatic Skill Selection

Use gstack skills as needed — the system determines which to run from *what you're building*, without being told the skill name:

- **End-user products:** `/plan-design-review` (before) → `/design-review` (after)
- **Developer tools:** `/plan-devex-review` (before) → `/devex-review` (after)
- **Architecture:** `/plan-eng-review` (before) → `/review` (after)
- **Everything:** `/autoplan` auto-detects the applicable reviews and surfaces only taste decisions needing approval.

Other gstack skills (auto-routed by intent): `/office-hours`, `/spec`, `/design-shotgun`, `/design-html`, `/qa`, `/investigate`, `/ship`, `/land-and-deploy`.

## 7. Global Rules (always apply)

The global rules in `~/.claude/CLAUDE.md` and their skills apply to every change in this repo — they are the source of truth, do not duplicate them here:

- **Rust — `/rust-skills`**: 179 rules across 14 categories (ownership, error handling, async, API design, memory, performance, testing, anti-patterns). ALL Rust written, reviewed, or refactored here must follow these. Consult before and during any Rust work.
- **Ponytail** (§5): the lazy-senior-dev ladder for every line; review the diff with `/ponytail-review` and the repo with `/ponytail-audit` after building.
- **gstack workflow** (§6): plan review up front for non-trivial features, `/review` before a PR, `/design-review` for UI.
- **db-standards — `/db-standards`**: applies to any SQL, tracking table schema, or migration file conventions added here.
- **humanizer — `/humanizer`**: applied to any user-facing prose or narration.

## 8. This Project — soma-schema

**soma-schema is a standalone Postgres migration tool** — plain SQL files with UP/DOWN sections, manifest-driven ordering, SHA-256 checksum drift detection, and advisory-lock safety. It ships as both a library crate (`soma_schema`) and a binary (`soma-schema`). The binary is behind the `cli` feature (on by default).

It is part of soma-platform and is intended to be used by soma-vault and other soma services, but is fully independent and usable standalone.

### Commands

```bash
# Check + unit tests (no Postgres needed)
cargo check
cargo test                          # runs unit tests in src/ only

# Integration tests (require a real Postgres instance)
TEST_DATABASE_URL=postgres://user:pass@localhost/mydb cargo test --test integration

# Run a single integration test by name
TEST_DATABASE_URL=... cargo test --test integration test_checksum_drift_error

# Build release binary
cargo build --release               # output: target/release/soma-schema

# CLI
cargo run -- --help
cargo run -- --database-url postgres://... up
cargo run -- --database-url postgres://... down --steps 2
cargo run -- --database-url postgres://... status
cargo run -- init                   # scaffold a new migrations directory

# Lint
cargo clippy -- -D warnings
```

Integration tests create and tear down throwaway Postgres schemas named `_sdm_test_<uuid>` — they never touch `public` or any pre-existing schema.

### Architecture

The library has four layers:

**1. Driver trait (`src/driver.rs`)**
`MigrationDriver` is the only abstraction point. It defines six async operations: `acquire_lock`, `run_setup_sql`, `ensure_tracking_table`, `applied`, `apply`, `revert`. `LockGuard` is a drop-based RAII handle that releases the advisory lock. Adding a new database backend means implementing `MigrationDriver`; no other code changes.

**2. Postgres implementation (`src/postgres.rs`)**
`PostgresDriver` implements `MigrationDriver` using sqlx. Two connections are always needed: one dedicated connection holds the advisory lock for the duration of the operation; the other does migration work. The pool is sized to exactly 2 in the CLI. `PgLockGuard` (Drop impl) releases the lock even on panic.

**3. Migrator (`src/migrator.rs`)**
`Migrator` owns only a root `PathBuf`. Its three public methods — `up`, `down`, `status` — orchestrate the full workflow: acquire lock → run setup → ensure tracking table → check applied checksums → apply/revert. Each migration's SQL is read once and passed directly to the driver; there is no TOCTOU window between checksum computation and execution. `scaffold` creates the initial directory structure.

**4. File-system layer (`src/discovery.rs`, `src/manifest.rs`, `src/migration.rs`)**

- `discover(root)` returns `(Vec<Migration>, Vec<SetupFile>)`. It reads `migration-order.yaml` via `Manifest::from_yaml`, then loads each listed SQL file in manifest order (version ascending, then entry order within each version). Version folders are sorted numerically, not lexicographically.
- `Migration` holds the raw SQL string. `read_up()` returns everything before `-- DOWN ==`; `read_down()` returns everything after. The checksum covers the entire raw file content.
- `Manifest` validates: `manifest_version == 1`, no path separators in filenames, no duplicate entries.

### Migration file layout

```text
migrations/
  migration-order.yaml          ← canonical manifest; defines order
  00_setup/                     ← idempotent bootstrap SQL; runs before every up()
    01_schema.sql
  01_migrated/                  ← applied migrations, organised by version number
    1/
      20260101_01_init.sql
      20260101_02_seed.sql
  02_inprogress/                ← optional staging area for in-flight work
```

SQL file format:

```sql
-- UP section (everything before the delimiter)
CREATE TABLE foo (id SERIAL PRIMARY KEY);

-- DOWN ==
DROP TABLE IF EXISTS foo;
```

The tracking table is named `00_schema_migrations` by default (the `00_` prefix sorts it before application tables in most GUIs). It records `version`, `file`, `checksum`, `batch`, `applied_at`, `applied_by`, `execution_ms`.

### Key invariants to preserve

- **Advisory lock scope**: the lock must be held for the entire `up`/`down` call, not per-migration. `acquire_lock` returns a `Box<dyn LockGuard>` held as `_lock` for the call's lifetime.
- **Checksum on raw content**: the checksum is SHA-256 of the full file bytes, UP+DOWN together. Changing the DOWN section of an applied migration is a drift error.
- **Apply + track in one transaction**: `apply` and `revert` in the driver must insert/delete the tracking row in the same transaction as the migration SQL. Separate transactions would leave the DB in a split state on crash.
- **Manifest order over filename order**: `down()` reverses the manifest position map, not filename sort. This is what makes FK-safe rollback deterministic.

---

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.

## graphify

This project has a graphify knowledge graph at graphify-out/.

Rules:
- Before answering architecture or codebase questions, read graphify-out/GRAPH_REPORT.md for god nodes and community structure
- If graphify-out/wiki/index.md exists, navigate it instead of reading raw files
- After modifying code files in this session, run `python3 -c "from graphify.watch import _rebuild_code; from pathlib import Path; _rebuild_code(Path('.'))"` to keep the graph current
