# Pull request

## What this changes and why

(Replace this line with a brief description.)

## Checklist

- [ ] `cargo test` passes (unit + doc tests)
- [ ] `cargo clippy --all-targets -- -D warnings` is clean
- [ ] `cargo fmt --check` passes
- [ ] Integration tests pass if database behavior changed (`TEST_DATABASE_URL=... cargo test --test integration`)
- [ ] Docs or comments updated where behavior changed
- [ ] No edits made to SQL files that are already tracked as applied migrations (changing applied files is a drift error by design)
