# Development — build & disk hygiene

## Why `target/` grows large

`target/` is Cargo's per-crate build cache. It is gitignored, disposable, never included in the published crate (~192 KB tarball), and never deployed to the website. It exists only on your local machine.

This repo has **three separate caches** because it contains three independent crates:

| Crate | Location | Notes |
|---|---|---|
| `soma-schema` (lib + CLI) | `target/` | Core library and binary |
| `website` (Leptos / WASM) | `website/target/` | WASM builds are the largest; can reach several GB |
| `explorer-data` (tool) | `tools/explorer-data/target/` | Small data-emit tool |

Each has its own `[profile.dev] debug = false` so dev builds skip debug-info emission — the single biggest driver of cache size. Backtraces on panics still print the message; only line-level source info is omitted.

## Reclaiming disk

Clean all three caches at once:

```sh
cargo clean
cargo clean --manifest-path website/Cargo.toml
cargo clean --manifest-path tools/explorer-data/Cargo.toml
```

Or with find (covers any nested crate added later):

```sh
find . -type d -name target -prune -exec rm -rf {} +
```

Also safe to delete — all gitignored and regenerated on next build or deploy:

```sh
rm -rf website/dist          # trunk build output
rm -rf docs/public           # built docs site
```

And local-only tooling artifacts (also gitignored):

```sh
rm -f ruvector.db            # local vector DB
rm -rf graphify-out/         # knowledge-graph output
rm -rf .playwright-mcp/      # browser test cache
```

## Build commands

```sh
# Check + unit tests (no Postgres required)
cargo check
cargo test

# Integration tests (real Postgres required)
TEST_DATABASE_URL=postgres://user:pass@localhost/mydb cargo test --test integration

# Release binary
cargo build --release

# WASM website (check only — avoids rebuilding the large WASM cache during dev)
cd website && cargo check --target wasm32-unknown-unknown

# WASM website (full build for deployment)
cd website && trunk build --release
```

## Optional: shared cache across all three crates

Set `CARGO_TARGET_DIR` (or `[build] target-dir` in `~/.cargo/config.toml`) to a single path and all three crates will write into one directory instead of three:

```toml
# ~/.cargo/config.toml
[build]
target-dir = "/path/to/shared/target"
```

Trade-off: faster cold builds across projects; the single directory can still grow large if you work on many crates. `cargo clean` there clears everything.
