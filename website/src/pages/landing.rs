use leptos::prelude::*;
use leptos_router::components::A;
use soma_ui::CodeBlock;
use soma_ui::{use_clipboard, UseClipboardReturn};

#[component]
pub fn LandingPage() -> impl IntoView {
    let UseClipboardReturn { copy, copied } = use_clipboard();
    let cmds = "cargo add soma-schema\nsoma-schema init migrations/\nsoma-schema --database-url $DATABASE_URL --migrations migrations/ up\nsoma-schema --database-url $DATABASE_URL --migrations migrations/ status\nsoma-schema --database-url $DATABASE_URL --migrations migrations/ down".to_string();
    let on_copy_qs = {
        let cmds = cmds.clone();
        move |_| copy.run(cmds.clone())
    };

    view! {
        <div class="page-atmosphere">
            // ── HERO ───────────────────────────────────────────────────────
            <section class="landing-container pt-20 pb-16 md:pt-28 md:pb-20">
                <div class="flex flex-col lg:flex-row items-start lg:items-center gap-12 lg:gap-16">
                    // Left: text content
                    <div class="flex-1 min-w-0">
                        <p class="hero-eyebrow">
                            "AI-native \u{00B7} Postgres \u{00B7} Rust"
                        </p>
                        <h1 class="hero-title">
                            "Schema control,\u{a0}"
                            <span class="accent-word">"without"</span>
                            "\u{a0}the drift."
                        </h1>
                        <p class="hero-subtitle mt-5">
                            "Drop one block into your AGENTS.md and any agent \u{2014} Claude, Cursor, Copilot \u{2014} follows the conventions (UP/DOWN, the manifest entry, FK-safe order). Agents slip; soma-schema\u{2019}s guardrails catch it \u{2014} checksum drift, bad ordering, editing a shipped file."
                        </p>
                        <div class="hero-ctas flex flex-wrap gap-3 mt-8">
                            <a
                                href="https://crates.io/crates/soma-schema"
                                target="_blank"
                                rel="noopener noreferrer"
                                class="btn-primary"
                                aria-label="Add soma-schema from crates.io"
                            >
                                "cargo add soma-schema"
                            </a>
                            <A href=crate::app::app_url("/explorer") attr:class="btn-ghost">
                                "Explore migrations \u{2192}"
                            </A>
                        </div>
                        // Badges
                        <div class="flex flex-wrap items-center gap-3 mt-4">
                            <a
                                href="https://github.com/chaitugsk07/soma-schema"
                                target="_blank"
                                rel="noopener noreferrer"
                                aria-label="GitHub stars for soma-schema"
                            >
                                <img
                                    src="https://img.shields.io/github/stars/chaitugsk07/soma-schema?style=flat-square"
                                    alt="GitHub stars"
                                    height="20"
                                />
                            </a>
                            <a
                                href="https://crates.io/crates/soma-schema"
                                target="_blank"
                                rel="noopener noreferrer"
                                aria-label="crates.io downloads for soma-schema"
                            >
                                <img
                                    src="https://img.shields.io/crates/d/soma-schema?style=flat-square"
                                    alt="crates.io downloads"
                                    height="20"
                                />
                            </a>
                        </div>
                    </div>

                    // Right: migration file visual
                    <div class="hero-visual flex-shrink-0 w-full lg:w-auto">
                        <div class="migration-file-card" aria-hidden="true">
                            <div class="card-chrome">
                                <span class="chrome-dot" style="background:#ff5f57"></span>
                                <span class="chrome-dot" style="background:#ffbd2e"></span>
                                <span class="chrome-dot" style="background:#28ca41"></span>
                                <span class="card-filename">"20260101_01_init.sql"</span>
                            </div>
                            <CodeBlock
                                code="-- UP: Create the widgets table\nCREATE TABLE IF NOT EXISTS example.widgets (\n    id          UUID        PRIMARY KEY\n                            DEFAULT gen_random_uuid(),\n    name        TEXT        NOT NULL,\n    created_at  TIMESTAMPTZ NOT NULL\n                            DEFAULT now()\n);\n\n-- DOWN ==\nDROP TABLE IF EXISTS example.widgets;".to_string()
                                language="sql".to_string()
                            />
                        </div>
                    </div>
                </div>
            </section>

            // ── ORIGIN STORY ───────────────────────────────────────────────
            <section class="page-section landing-container ss-anim-1" aria-labelledby="origin-heading">
                <div class="origin-story">
                    <span class="origin-eyebrow" id="origin-heading">"Why I built this"</span>
                    <div class="origin-body">
                        <p>
                            "I build a lot with AI coding agents now, and the database layer is where it kept falling apart. The agent would write a migration that ran out of order, edit one that was already applied, skip the rollback, or quietly break a foreign-key drop \u{2014} and nothing I tried was built for that workflow. Every migration tool assumed a human typing commands, not an agent generating SQL."
                        </p>
                        <p>
                            "So I built the one I wanted: the rules an agent needs written down (drop an AGENTS.md in and it follows them), guardrails that catch the mistakes it still makes \u{2014} a changed file that was already applied, broken ordering, a missing rollback \u{2014} before they reach the database, and a visual explorer so I can actually see the schema and seed data it produced. Agents are non-deterministic; soma-schema is the safety net I wish I\u{2019}d had when I started building with them."
                        </p>
                    </div>
                </div>
            </section>

            <div class="ss-separator landing-container"></div>

            // ── AI-NATIVE ──────────────────────────────────────────────────
            {
                let agent_rules = "## Database migrations — soma-schema\n\nThis project uses soma-schema for all database migrations.\nDocs: https://chaitugsk07.github.io/soma-schema · Repo: https://github.com/chaitugsk07/soma-schema\n\nWhen asked to change the database schema, generate a soma-schema migration. Do NOT\nhand-edit the database or any already-applied migration file.\n\nHow migrations work here:\n- Plain SQL files under migrations/01_migrated/<version>/, each with an UP section,\n  a line that trims to exactly \"-- DOWN ==\", then a DOWN section.\n- Every file is listed in migrations/migration-order.yaml, which defines apply order\n  (top to bottom) and rollback order (the exact reverse).\n- The runner checksums the whole file (UP+DOWN) and applies + records each migration\n  in one transaction, under a database advisory lock.\n\nRules when generating a migration:\n1. NEVER edit a migration that was already applied — checksum drift errors the next run.\n   To change deployed schema, write a NEW migration.\n2. Add every new .sql to migration-order.yaml in the correct version block, in apply order.\n3. Write a DOWN that undoes UP in FK-safe reverse order (drop children before parents).\n4. Seeds are idempotent: UP uses ON CONFLICT DO NOTHING so re-runs are safe.\n5. One schema per service; 00_setup/ must CREATE SCHEMA IF NOT EXISTS it (idempotent only).\n6. Follow this project's existing SQL conventions (naming, types, allowed extensions).\n\nTo add a migration:\n- Create migrations/01_migrated/<version>/<YYYYMMDD>_<NN>_<name>.sql with UP + \"-- DOWN ==\" + DOWN.\n- Add it to migration-order.yaml (created/author/why).\n- Run: soma-schema --migrations migrations status   (confirm it's pending)\n- Run: soma-schema --migrations migrations up        (apply it)\n- Never touch the file again once applied.".to_string();
                view! {
                    <section class="page-section landing-container ss-anim-1">
                        <span class="section-eyebrow ai-native-eyebrow">"AI-native"</span>
                        <h2 class="section-title">"Your agent writes the migrations"</h2>
                        <p class="ai-native-lead">
                            "Drop these rules into your agent\u{2019}s instructions file and it follows the conventions on its own \u{2014} proper UP/DOWN, the manifest entry, FK-safe order. Agents are non-deterministic, so soma-schema also enforces the invariants: it errors on checksum drift if an applied file changes, rejects out-of-order or orphaned migrations, and keeps rollback FK-safe. Works with any agentic coding tool: Claude Code, OpenAI Codex, Cursor, Windsurf, GitHub Copilot, Google Antigravity, Aider, and more."
                        </p>
                        <div class="mt-6">
                            <CodeBlock
                                code=agent_rules
                                filename="AGENTS.md".to_string()
                            />
                        </div>
                        <div class="agent-rules-map mt-5">
                            <ul class="agent-rules-list">
                                <li>
                                    <span class="agent-rules-file">"AGENTS.md"</span>
                                    <span class="agent-rules-sep">" \u{2014} "</span>
                                    <span class="agent-rules-desc">"cross-tool standard (OpenAI Codex, Cursor, Zed, Google Antigravity, and most agentic tools)"</span>
                                </li>
                                <li>
                                    <span class="agent-rules-file">"CLAUDE.md"</span>
                                    <span class="agent-rules-sep">" \u{2014} "</span>
                                    <span class="agent-rules-desc">"Claude Code"</span>
                                </li>
                                <li>
                                    <span class="agent-rules-file">".cursor/rules/*.mdc"</span>
                                    <span class="agent-rules-sep">" \u{2014} "</span>
                                    <span class="agent-rules-desc">"Cursor (legacy: .cursorrules)"</span>
                                </li>
                                <li>
                                    <span class="agent-rules-file">".windsurf/rules/"</span>
                                    <span class="agent-rules-sep">" \u{2014} "</span>
                                    <span class="agent-rules-desc">"Windsurf (legacy: .windsurfrules)"</span>
                                </li>
                                <li>
                                    <span class="agent-rules-file">".github/copilot-instructions.md"</span>
                                    <span class="agent-rules-sep">" \u{2014} "</span>
                                    <span class="agent-rules-desc">"GitHub Copilot"</span>
                                </li>
                                <li>
                                    <span class="agent-rules-file">"CONVENTIONS.md"</span>
                                    <span class="agent-rules-sep">" \u{2014} "</span>
                                    <span class="agent-rules-desc">"Aider"</span>
                                </li>
                            </ul>
                            <p class="agent-rules-tip">
                                "Same rules \u{2014} just put them in the file your tool reads. Keep AGENTS.md as the source of truth and have the others reference it."
                            </p>
                        </div>
                        <p class="ai-native-skill-hint mt-5">
                            "Claude Code users also get a /soma-schema slash-command skill \u{2014} "
                            <a
                                href=crate::app::docs_url("use-with-ai/")
                                class="ai-native-docs-link"
                            >
                                "see the docs \u{2192}"
                            </a>
                        </p>
                    </section>
                }
            }

            <div class="ss-separator landing-container"></div>

            // ── ONE COMMAND ────────────────────────────────────────────────
            <section class="page-section landing-container ss-anim-2" aria-labelledby="one-cmd-heading">
                <span class="section-eyebrow init-eyebrow">"One command"</span>
                <h2 class="section-title" id="one-cmd-heading">
                    "One command. Your agent\u{2019}s wired up."
                </h2>
                <p class="init-lead">
                    "Install the CLI, scaffold migrations, drop in agent rules, and get a runnable example \u{2014} all at once."
                </p>

                // One-liner terminal block
                <div class="terminal-card mt-6 init-oneliner-card" aria-label="One-command init">
                    <div class="terminal-chrome">
                        <span class="chrome-dot" style="background:#ff5f57"></span>
                        <span class="chrome-dot" style="background:#ffbd2e"></span>
                        <span class="chrome-dot" style="background:#28ca41"></span>
                        <span class="chrome-title">"terminal"</span>
                    </div>
                    <div class="terminal-body">
                        <div>
                            <span class="t-prompt">"$ "</span>
                            <span class="t-cmd">"cargo install soma-schema "</span>
                            <span class="t-flag">"&& "</span>
                            <span class="t-cmd">"soma-schema init"</span>
                        </div>
                    </div>
                </div>

                // ── FLOW GRAPH ─────────────────────────────────────────────
                <div class="init-flow-wrap mt-8" role="img" aria-label="init command flow: cargo install → soma-schema init → scaffolds migrations directory, writes AGENTS.md, creates runnable example migration → then run up and explorer">
                    // Desktop SVG flow (hidden on mobile)
                    <div class="init-flow-desktop" aria-hidden="true">
                        <svg class="init-flow-svg" viewBox="0 0 960 340" xmlns="http://www.w3.org/2000/svg" role="presentation" focusable="false">
                            // ── Background panel
                            <rect x="0" y="0" width="960" height="340" rx="14" class="init-flow-bg"/>

                            // ── Step 0: cargo install ──────────────────────
                            <rect x="24" y="130" width="160" height="80" rx="10" class="init-node init-node-install init-node-anim-0"/>
                            <text x="104" y="163" text-anchor="middle" class="init-node-label init-node-anim-0">"cargo install"</text>
                            <text x="104" y="181" text-anchor="middle" class="init-node-mono init-node-anim-0">"soma-schema"</text>
                            <text x="104" y="198" text-anchor="middle" class="init-node-sub init-node-anim-0">"crates.io"</text>

                            // Arrow install → init
                            <line x1="184" y1="170" x2="232" y2="170" class="init-connector init-conn-anim-0"/>
                            <polygon points="232,165 242,170 232,175" class="init-arrow init-conn-anim-0"/>

                            // ── Step 1: soma-schema init (central node) ────
                            <rect x="242" y="110" width="190" height="120" rx="10" class="init-node init-node-center init-node-anim-1"/>
                            <text x="337" y="158" text-anchor="middle" class="init-node-center-label init-node-anim-1">"soma-schema"</text>
                            <text x="337" y="178" text-anchor="middle" class="init-node-center-cmd init-node-anim-1">"init"</text>
                            <text x="337" y="205" text-anchor="middle" class="init-node-sub init-node-anim-1">"--rules agents (default)"</text>

                            // Branch connectors from center to three outputs
                            // Top branch → migrations/
                            <line x1="432" y1="150" x2="510" y2="80" class="init-connector init-conn-branch init-conn-anim-1"/>
                            <polygon points="506,75 516,80 507,86" class="init-arrow-branch init-conn-anim-1"/>
                            // Mid branch → AGENTS.md
                            <line x1="432" y1="170" x2="510" y2="170" class="init-connector init-conn-branch init-conn-anim-1"/>
                            <polygon points="510,165 520,170 510,175" class="init-arrow-branch init-conn-anim-1"/>
                            // Bot branch → example migration
                            <line x1="432" y1="190" x2="510" y2="260" class="init-connector init-conn-branch init-conn-anim-1"/>
                            <polygon points="506,254 516,260 508,266" class="init-arrow-branch init-conn-anim-1"/>

                            // ── Output A: migrations/ tree ─────────────────
                            <rect x="520" y="30" width="210" height="140" rx="10" class="init-node init-node-fs init-node-anim-2"/>
                            <text x="533" y="56" class="init-node-label-left init-node-anim-2">"migrations/"</text>
                            <text x="541" y="77" class="init-node-mono-sm init-node-anim-2">"migration-order.yaml"</text>
                            <text x="541" y="97" class="init-node-mono-sm init-node-anim-2">"00_setup/"</text>
                            <text x="553" y="115" class="init-node-mono-sm init-node-mono-muted init-node-anim-2">"01_schema.sql"</text>
                            <text x="541" y="133" class="init-node-mono-sm init-node-anim-2">"01_migrated/1/"</text>
                            <text x="553" y="151" class="init-node-mono-sm init-node-mono-muted init-node-anim-2">"20260101_01_example.sql"</text>

                            // ── Output B: AGENTS.md ───────────────────────
                            <rect x="520" y="130" width="210" height="80" rx="10" class="init-node init-node-agents init-node-anim-3"/>
                            <text x="625" y="163" text-anchor="middle" class="init-node-label init-node-anim-3">"AGENTS.md"</text>
                            <text x="625" y="181" text-anchor="middle" class="init-node-sub init-node-anim-3">"agent rules wired up"</text>
                            <text x="625" y="198" text-anchor="middle" class="init-node-sub init-node-anim-3">"--rules agents|claude|all"</text>

                            // ── Output C: runnable example ─────────────────
                            <rect x="520" y="220" width="210" height="90" rx="10" class="init-node init-node-example init-node-anim-4"/>
                            <text x="625" y="252" text-anchor="middle" class="init-node-label init-node-anim-4">"example migration"</text>
                            <text x="625" y="270" text-anchor="middle" class="init-node-mono init-node-anim-4">"CREATE TABLE … UP"</text>
                            <text x="625" y="288" text-anchor="middle" class="init-node-mono init-node-anim-4">"DROP TABLE … DOWN"</text>

                            // ── Merge connector → next steps ───────────────
                            // top + mid merge line
                            <line x1="730" y1="100" x2="770" y2="100" class="init-connector init-conn-merge init-conn-anim-4"/>
                            <line x1="730" y1="170" x2="750" y2="170" class="init-connector init-conn-merge init-conn-anim-4"/>
                            <line x1="730" y1="265" x2="750" y2="265" class="init-connector init-conn-merge init-conn-anim-4"/>
                            // vertical bar collecting all three
                            <line x1="750" y1="100" x2="750" y2="265" class="init-connector init-conn-merge init-conn-anim-4"/>
                            // horizontal to next-steps box
                            <line x1="750" y1="183" x2="778" y2="183" class="init-connector init-conn-merge init-conn-anim-4"/>
                            <polygon points="778,178 788,183 778,188" class="init-arrow-merge init-conn-anim-4"/>

                            // ── Next steps: up + explorer ──────────────────
                            <rect x="788" y="120" width="148" height="126" rx="10" class="init-node init-node-next init-node-anim-5"/>
                            <text x="862" y="152" text-anchor="middle" class="init-node-label-next init-node-anim-5">"what\u{2019}s next"</text>
                            <text x="862" y="175" text-anchor="middle" class="init-node-mono-next init-node-anim-5">"soma-schema up"</text>
                            <text x="862" y="193" text-anchor="middle" class="init-node-sub init-node-anim-5">"apply migrations"</text>
                            <text x="862" y="218" text-anchor="middle" class="init-node-mono-next init-node-anim-5">"soma-schema"</text>
                            <text x="862" y="236" text-anchor="middle" class="init-node-mono-next init-node-anim-5">"explorer"</text>
                        </svg>
                    </div>

                    // Mobile flow (stacked cards, shown only on small screens)
                    <div class="init-flow-mobile" aria-hidden="true">
                        <div class="init-flow-step init-node-install">
                            <span class="init-flow-step-label">"install"</span>
                            <code class="init-flow-step-cmd">"cargo install soma-schema"</code>
                        </div>
                        <div class="init-flow-arrow-down" aria-hidden="true">"↓"</div>
                        <div class="init-flow-step init-node-center-mobile">
                            <span class="init-flow-step-label">"scaffold"</span>
                            <code class="init-flow-step-cmd">"soma-schema init"</code>
                        </div>
                        <div class="init-flow-arrow-down" aria-hidden="true">"↓"</div>
                        <div class="init-flow-mobile-outputs">
                            <div class="init-flow-output init-node-fs-mobile">
                                <span class="init-flow-out-icon" aria-hidden="true">"📁"</span>
                                <div>
                                    <div class="init-flow-out-title">"migrations/"</div>
                                    <div class="init-flow-out-sub">"order.yaml · 00_setup/ · 01_migrated/1/…"</div>
                                </div>
                            </div>
                            <div class="init-flow-output init-node-agents-mobile">
                                <span class="init-flow-out-icon" aria-hidden="true">"🤖"</span>
                                <div>
                                    <div class="init-flow-out-title">"AGENTS.md"</div>
                                    <div class="init-flow-out-sub">"agent rules · --rules agents|claude|all"</div>
                                </div>
                            </div>
                            <div class="init-flow-output init-node-example-mobile">
                                <span class="init-flow-out-icon" aria-hidden="true">"⚡"</span>
                                <div>
                                    <div class="init-flow-out-title">"example migration"</div>
                                    <div class="init-flow-out-sub">"CREATE TABLE UP · DROP TABLE DOWN"</div>
                                </div>
                            </div>
                        </div>
                        <div class="init-flow-arrow-down" aria-hidden="true">"↓"</div>
                        <div class="init-flow-step init-node-next-mobile">
                            <span class="init-flow-step-label">"then"</span>
                            <code class="init-flow-step-cmd">"soma-schema up"</code>
                            <code class="init-flow-step-cmd">"soma-schema explorer"</code>
                        </div>
                    </div>
                </div>
            </section>

            <div class="ss-separator landing-container"></div>

            // ── QUICKSTART ─────────────────────────────────────────────────
            <section class="page-section landing-container ss-anim-2">
                <span class="section-eyebrow">"Quickstart"</span>
                <h2 class="section-title">"Up and running in minutes"</h2>
                <div class="terminal-card mt-6" style="position: relative;" role="region" aria-label="Quickstart terminal commands">
                    <div class="terminal-chrome">
                        <span class="chrome-dot" style="background:#ff5f57"></span>
                        <span class="chrome-dot" style="background:#ffbd2e"></span>
                        <span class="chrome-dot" style="background:#28ca41"></span>
                        <span class="chrome-title">"terminal"</span>
                    </div>
                    <button
                        class="terminal-copy-btn"
                        aria-label="Copy commands"
                        on:click=on_copy_qs
                    >
                        {move || if copied.get() { "✓" } else { "⧉" }}
                    </button>
                    <div class="terminal-body">
                        <div>
                            <span class="t-comment">"# 1. Install the CLI and scaffold in one shot"</span>
                        </div>
                        <div>
                            <span class="t-prompt">"$ "</span>
                            <span class="t-cmd">"cargo install soma-schema "</span>
                            <span class="t-flag">"&& "</span>
                            <span class="t-cmd">"soma-schema init"</span>
                        </div>
                        <div class="mt-3">
                            <span class="t-comment">"# 2. Apply everything pending"</span>
                        </div>
                        <div>
                            <span class="t-prompt">"$ "</span>
                            <span class="t-cmd">"soma-schema "</span>
                            <span class="t-flag">"--database-url "</span>
                            <span class="t-arg">"$DATABASE_URL "</span>
                            <span class="t-cmd">"up"</span>
                        </div>
                        <div class="mt-3">
                            <span class="t-comment">"# 3. Check status"</span>
                        </div>
                        <div>
                            <span class="t-prompt">"$ "</span>
                            <span class="t-cmd">"soma-schema "</span>
                            <span class="t-flag">"--database-url "</span>
                            <span class="t-arg">"$DATABASE_URL "</span>
                            <span class="t-cmd">"status"</span>
                        </div>
                        <div class="mt-3">
                            <span class="t-comment">"# 4. Roll back last migration"</span>
                        </div>
                        <div>
                            <span class="t-prompt">"$ "</span>
                            <span class="t-cmd">"soma-schema "</span>
                            <span class="t-flag">"--database-url "</span>
                            <span class="t-arg">"$DATABASE_URL "</span>
                            <span class="t-cmd">"down"</span>
                        </div>
                        <div class="mt-3">
                            <span class="t-comment">"# 5. Open the visual explorer"</span>
                        </div>
                        <div>
                            <span class="t-prompt">"$ "</span>
                            <span class="t-cmd">"soma-schema explorer"</span>
                        </div>
                    </div>
                </div>
            </section>

            <div class="ss-separator landing-container"></div>

            // ── BENTO FEATURES ─────────────────────────────────────────────
            <section class="page-section landing-container ss-anim-2">
                <span class="section-eyebrow">"Why soma-schema"</span>
                <h2 class="section-title">"Four invariants. Zero compromises."</h2>
                <div class="bento-grid mt-8">
                    // Featured: Manifest ordering
                    <div class="bento-card bento-card-featured">
                        <span class="bento-icon" aria-hidden="true">
                            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <polyline points="8 6 21 6"/>
                                <polyline points="8 12 21 12"/>
                                <polyline points="8 18 21 18"/>
                                <line x1="3" y1="6" x2="3.01" y2="6"/>
                                <line x1="3" y1="12" x2="3.01" y2="12"/>
                                <line x1="3" y1="18" x2="3.01" y2="18"/>
                            </svg>
                        </span>
                        <p class="bento-label">"Ordering"</p>
                        <p class="bento-title">"Manifest-defined order"</p>
                        <p class="bento-desc">
                            "migration-order.yaml lists every migration explicitly. Apply order is top-to-bottom; rollback is the exact reverse — not filename sort. Deterministic FK-safe rollback without naming conventions."
                        </p>
                    </div>

                    // Drift detection
                    <div class="bento-card bento-card-drift">
                        <span class="bento-icon" aria-hidden="true">
                            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
                            </svg>
                        </span>
                        <p class="bento-label">"Integrity"</p>
                        <p class="bento-title">"Full-file checksum drift"</p>
                        <p class="bento-desc">
                            "SHA-256 covers the entire file — UP and DOWN together. Editing the DOWN section of a deployed migration is caught as ChecksumDrift before anything runs."
                        </p>
                    </div>

                    // Atomic apply
                    <div class="bento-card bento-card-forward">
                        <span class="bento-icon" aria-hidden="true">
                            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>
                            </svg>
                        </span>
                        <p class="bento-label">"Atomicity"</p>
                        <p class="bento-title">"Apply + track in one transaction"</p>
                        <p class="bento-desc">
                            "The migration SQL and its tracking-table row commit atomically. A crash between those two operations is not possible — no split state, ever."
                        </p>
                    </div>

                    // Advisory lock
                    <div class="bento-card bento-card-rollback">
                        <span class="bento-icon" aria-hidden="true">
                            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
                                <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
                            </svg>
                        </span>
                        <p class="bento-label">"Safety"</p>
                        <p class="bento-title">"Run-scoped advisory lock"</p>
                        <p class="bento-desc">
                            "A Postgres advisory lock is acquired at the start of up, down, or status and held via RAII until the call returns — even on panic. Concurrent runners block, never collide."
                        </p>
                    </div>
                </div>
            </section>

            <div class="ss-separator landing-container"></div>

            // ── COMPARISON TABLE ───────────────────────────────────────────
            <section class="page-section landing-container ss-anim-3">
                <span class="section-eyebrow">"Comparison"</span>
                <h2 class="section-title">"How it stacks up"</h2>
                <div class="spec-table-wrap mt-6">
                    <table>
                        <thead>
                            <tr>
                                <th>"Tool"</th>
                                <th>"Lang"</th>
                                <th>"Ordering"</th>
                                <th>"Checksum"</th>
                                <th>"Locking"</th>
                                <th>"Lib+CLI"</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr class="spec-row-highlight">
                                <td>"soma-schema"</td>
                                <td>"Rust"</td>
                                <td>"YAML manifest"</td>
                                <td>"Full-file UP+DOWN"</td>
                                <td>"Advisory, full-run"</td>
                                <td><span style="color: hsl(var(--forward))">"\u{2713}"</span></td>
                            </tr>
                            <tr>
                                <td>"sqlx migrate"</td>
                                <td>"Rust"</td>
                                <td>"Filename lexical"</td>
                                <td>"UP-only"</td>
                                <td>"Advisory (Pg)"</td>
                                <td><span style="color: hsl(var(--forward))">"\u{2713}"</span></td>
                            </tr>
                            <tr>
                                <td>"refinery"</td>
                                <td>"Rust"</td>
                                <td>"Filename lexical"</td>
                                <td>"Metadata hash"</td>
                                <td><span style="color: hsl(var(--muted-foreground)); opacity: 0.5">"\u{2717}"</span></td>
                                <td>"Lib only"</td>
                            </tr>
                            <tr>
                                <td>"diesel_migrations"</td>
                                <td>"Rust"</td>
                                <td>"Filename sort"</td>
                                <td><span style="color: hsl(var(--muted-foreground)); opacity: 0.5">"\u{2717}"</span></td>
                                <td><span style="color: hsl(var(--muted-foreground)); opacity: 0.5">"\u{2717}"</span></td>
                                <td>"Lib (ORM)"</td>
                            </tr>
                            <tr>
                                <td>"dbmate"</td>
                                <td>"Go"</td>
                                <td>"Filename sort"</td>
                                <td><span style="color: hsl(var(--muted-foreground)); opacity: 0.5">"\u{2717}"</span></td>
                                <td>"Advisory"</td>
                                <td>"CLI only"</td>
                            </tr>
                            <tr>
                                <td>"Flyway"</td>
                                <td>"JVM"</td>
                                <td>"Version prefix"</td>
                                <td>"Full-file"</td>
                                <td>"Advisory"</td>
                                <td><span style="color: hsl(var(--forward))">"\u{2713}"</span></td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </section>

            <div class="ss-separator landing-container"></div>

            // ── WHY SOMA-SCHEMA ────────────────────────────────────────────
            <section class="page-section landing-container ss-anim-5">
                <span class="section-eyebrow">"Why soma-schema"</span>
                <h2 class="section-title">"Not your old migration tool"</h2>
                <p class="why-lead">
                    "Flyway, Liquibase, sqlx-migrate, Alembic were built for humans typing commands. soma-schema is built for a codebase where an AI agent writes most of the migrations."
                </p>
                <div class="why-grid mt-8">
                    <div class="why-card">
                        <p class="why-card-title">"AI-native"</p>
                        <p class="why-card-body">
                            "Ships ready-to-paste agent rules and a Claude skill so your agent follows the conventions instead of guessing \u{2014} and when it still slips (re-ordering, forgetting the DOWN, editing a shipped file), the checks catch it."
                        </p>
                    </div>
                    <div class="why-card">
                        <p class="why-card-title">"Explicit order, not filenames"</p>
                        <p class="why-card-body">
                            "A YAML manifest defines apply and rollback order; rollback is the exact reverse, so FK-dependent drops run in the right sequence. Filename-sorted tools break when naming drifts."
                        </p>
                    </div>
                    <div class="why-card">
                        <p class="why-card-title">"Whole-file drift detection"</p>
                        <p class="why-card-body">
                            "SHA-256 over UP+DOWN together. Edit a shipped migration, even a comment, and the next run stops. Most tools hash only UP, or nothing."
                        </p>
                    </div>
                    <div class="why-card">
                        <p class="why-card-title">"Crash- and concurrency-safe"</p>
                        <p class="why-card-body">
                            "Each migration and its tracking row commit in one transaction, under a run-scoped advisory lock. No half-applied state, no two runners colliding."
                        </p>
                    </div>
                    <div class="why-card">
                        <p class="why-card-title">"A library and a CLI, no runtime"</p>
                        <p class="why-card-body">
                            "Embed it in your Rust service to migrate at startup, or run the binary in CI. No JVM, no daemon."
                        </p>
                    </div>
                    <div class="why-card">
                        <p class="why-card-title">"One tool, many databases"</p>
                        <p class="why-card-body">
                            "Built on a pluggable MigrationDriver: PostgreSQL today, SQLite next, more backends via the driver interface."
                        </p>
                    </div>
                </div>
            </section>

            <div class="ss-separator landing-container"></div>

            // ── ROADMAP ────────────────────────────────────────────────────
            <section class="page-section landing-container ss-anim-4">
                <span class="section-eyebrow">"Roadmap"</span>
                <h2 class="section-title">"What\u{2019}s next"</h2>
                <p class="roadmap-lead">
                    "PostgreSQL is stable and production-ready. The driver architecture makes every other backend a clean implementation problem \u{2014} no core changes needed. "
                    <a
                        href="https://github.com/chaitugsk07/soma-schema/blob/main/ROADMAP.md"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="roadmap-full-link"
                    >
                        "Full roadmap \u{2192}"
                    </a>
                </p>

                // ── Backend tiers ──────────────────────────────────────────
                <div class="phase-strip phase-strip-4" role="list" aria-label="Database backend roadmap">
                    <div class="phase-item phase-live" role="listitem">
                        <p class="phase-label">"Now \u{00B7} Stable"</p>
                        <ul class="phase-db-list">
                            <li>"PostgreSQL"</li>
                        </ul>
                    </div>
                    <div class="phase-item phase-next" role="listitem">
                        <p class="phase-label">"Next"</p>
                        <ul class="phase-db-list">
                            <li>"SQLite"</li>
                        </ul>
                    </div>
                    <div class="phase-item" role="listitem">
                        <p class="phase-label">"Planned"</p>
                        <ul class="phase-db-list">
                            <li>"MySQL / MariaDB"</li>
                            <li>"CockroachDB"</li>
                        </ul>
                    </div>
                    <div class="phase-item phase-exploring" role="listitem">
                        <p class="phase-label">"Exploring"</p>
                        <ul class="phase-db-list">
                            <li>"SurrealDB"</li>
                            <li>"MongoDB"</li>
                            <li>"DuckDB"</li>
                        </ul>
                    </div>
                </div>

                // ── MigrationDriver note ───────────────────────────────────
                <p class="roadmap-driver-note">
                    "Any backend via the "
                    <code class="roadmap-code">"MigrationDriver"</code>
                    " trait \u{2014} contributions welcome."
                </p>

                // ── Features coming ────────────────────────────────────────
                <div class="roadmap-features-card" aria-label="Features coming soon">
                    <p class="roadmap-features-label">"Features coming"</p>
                    <ul class="roadmap-features-list" role="list">
                        <li>
                            <code class="roadmap-code">"dry-run"</code>
                            " \u{2014} preview changes without applying"
                        </li>
                        <li>
                            <code class="roadmap-code">"up --steps N"</code>
                            " \u{2014} apply exactly N migrations"
                        </li>
                        <li>
                            <code class="roadmap-code">"status --json"</code>
                            " \u{2014} machine-readable output for CI"
                        </li>
                        <li>
                            <code class="roadmap-code">"--lock-timeout"</code>
                            " \u{2014} fail fast instead of blocking forever"
                        </li>
                        <li>
                            <code class="roadmap-code">"verify"</code>
                            " \u{2014} re-checksum all applied migrations"
                        </li>
                        <li>
                            <code class="roadmap-code">"repair / baseline"</code>
                            " \u{2014} recover from manual schema changes"
                        </li>
                        <li>
                            <code class="roadmap-code">"new"</code>
                            " \u{2014} scaffold a timestamped migration file"
                        </li>
                    </ul>
                </div>
            </section>

            <div class="pb-16"></div>
        </div>
    }
}
