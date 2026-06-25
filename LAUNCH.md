# Launch checklist

This is the manual checklist for the 0.1.0 open-source launch. These steps are intentional, outward-facing, and mostly irreversible — they are NOT run by CI. Work through them in order.

---

## Pre-flight

- [ ] Confirm `LICENSE` (Apache-2.0) is present at the repo root.
- [ ] Run `cargo publish --dry-run` locally and confirm the file list is clean (no website/, docs/, tools/, graphify-out/, .db files, .rlib files).
- [ ] CI is green on the `main` branch (all check + integration jobs pass).
- [ ] Tag the release commit: `git tag v0.1.0 && git push origin v0.1.0`.
- [ ] Confirm the GitHub release is drafted (or create one from the tag) with the release notes paragraph below pasted in.

---

## Publish to crates.io

```bash
cargo publish
```

crates.io will automatically index the crate on lib.rs and trigger a docs.rs build. Docs are live at <https://docs.rs/soma-schema> once the build finishes (usually a few minutes).

---

## Enable GitHub Pages

1. Go to Settings → Pages → Source: select "GitHub Actions".
2. Re-run (or wait for) the "Deploy to GitHub Pages" workflow to build and deploy.
3. Confirm the site is live at <https://chaitugsk07.github.io/soma-schema/>.

---

## GitHub repo metadata

Set the following in the repository "About" panel (the gear icon on the main page):

- **Description**: "Plain SQL migrations for Rust — manifest-ordered, full-file drift detection, run-scoped advisory locking. Postgres today, multi-DB roadmap."
- **Website**: <https://chaitugsk07.github.io/soma-schema/>
- **Topics**: `rust`, `postgres`, `database`, `migrations`, `sql`, `cli`, `schema`

---

## Submission targets

Work through these in the order listed. Each line has the exact URL or instruction.

### Community aggregators

- **awesome-rust** — open a PR at <https://github.com/rust-unofficial/awesome-rust> adding soma-schema under the "Database" → "SQL" section. Link to <https://crates.io/crates/soma-schema>.
- **This Week in Rust / Crate of the Week** — post a nomination comment in the current week's thread at <https://users.rust-lang.org/t/crate-of-the-week>. One sentence is enough: name + one-line pitch.

### Developer directories

- **AlternativeTo** — add soma-schema at <https://alternativeto.net/software/add/> as an alternative to Flyway and Liquibase. Category: Database Tools.
- **Console.dev** — submit via <https://console.dev/tools/submit/>. Console curates developer tools; the CLI angle is a good fit.

### Newsletters

- **Postgres Weekly** — submit at <https://postgresweekly.com/issues>. Use the "Suggest a link" button at the bottom of any issue.
- **DB Weekly** — submit at <https://dbweekly.com/issues> (same publisher, same flow).

### Social

- **Show HN** — post on Hacker News at <https://news.ycombinator.com/submit>. Title: "Show HN: soma-schema – plain SQL migrations for Rust with full-file drift detection". Include the GitHub link.
- **r/rust** — post at <https://www.reddit.com/r/rust/> using the "Projects / Show and Tell" flair.
- **r/PostgreSQL** — cross-post at <https://www.reddit.com/r/PostgreSQL/>.

### Real-time communities

- **Rust Discord** — post in #crate-announcements at <https://discord.gg/rust-lang>.
- **rust-lang Zulip** — post in #wg-database stream at <https://rust-lang.zulipchat.com/>.

---

## AI-developer wave (run in parallel with Rust/Postgres channels)

The AI-native angle is the sharpest differentiator. soma-schema ships the agent contract so your AI can write correct migrations from the start — traditional tools don't have this. Target these channels with that framing:

**Angle:** "I built a migration tool that ships the agent rules so your AI writes correct migrations. Paste this block into your AGENTS.md and it just works."

### AI communities

- **Claude Discord** — post in the developer/tools channel at <https://discord.gg/anthropic>. Frame it around the AGENTS.md/CLAUDE.md block.
- **Cursor Discord / Forums** — post in #show-and-tell or the tools channel at <https://forum.cursor.com/>. Cursor users are primed for "works with your AI" tools.
- **r/ClaudeAI** — post at <https://www.reddit.com/r/ClaudeAI/> with the pasteable CLAUDE.md block front and center.
- **r/LocalLLaMA (dev channels)** — cross-post at <https://www.reddit.com/r/LocalLLaMA/> targeting the builder/dev-tools thread.

### AI newsletters

- **Latent Space** — submit a link or short write-up at <https://www.latent.space/>. The AI-engineering angle fits their audience well.
- **AI Engineer newsletter / Swyx's channels** — submit at <https://www.aiengineer.com/> or post in the AI Engineer Discord.

### Framing for AI-dev posts

Lead with the problem: "Every other migration tool was built for humans typing commands. You still have to tell your AI what invariants to follow, and it still gets them wrong." Then the solution: "soma-schema ships a ready-to-paste AGENTS.md block. Drop it in and your agent knows never to edit applied files, how to write idempotent seeds, how to order FK-safe rollbacks." Close with the link and the one-liner pitch.

---

## Release notes / announcement paragraph

Paste this (or adapt it) wherever release notes are expected:

> soma-schema is a plain SQL migration tool for Rust — no DSL, no ORM tie-in, no JVM. Migrations are ordinary `.sql` files with an `-- DOWN ==` delimiter, ordered by a YAML manifest rather than filename sort. This gives you deterministic FK-safe rollback order by construction. What sets it apart from the existing Rust migration crates: it checksums the entire file (UP and DOWN together), so editing a deployed DOWN section is caught before it silently breaks rollback; it holds a Postgres advisory lock for the whole `up`/`down` call, not per-migration, so concurrent runners can never interleave; and it ships as both a library crate and a standalone CLI. SQLite is next on the roadmap via the `MigrationDriver` trait — adding a backend means implementing six async methods, nothing else.

---

## After launch

- Monitor crates.io download stats and docs.rs build status.
- Watch GitHub issues for early bug reports.
- Reply to any HN or Reddit comments within 24 hours.
