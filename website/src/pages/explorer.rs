use leptos::prelude::*;
use soma_ui::{CodeBlock, SchemaDiagram, Table, TableBody, TableCell, TableHead, TableHeader, TableRow};
use crate::data::{load_migrations, load_schema, load_seed_data, MigrationEntry, SeedTable, VersionBlock};

// ── Single collapsible migration card ───────────────────────────────────────
#[component]
fn MigrationCard(order_num: usize, migration: MigrationEntry) -> impl IntoView {
    let open = RwSignal::new(false);
    let tab = RwSignal::new("up".to_string());

    let up_sql = StoredValue::new(migration.up_sql.clone());
    let down_sql = StoredValue::new(migration.down_sql.clone());
    let file = StoredValue::new(migration.file.clone());
    let why = StoredValue::new(migration.why.clone());
    let created = StoredValue::new(migration.created.clone());
    let author = StoredValue::new(migration.author.clone());
    let checksum_short = migration.checksum.chars().take(10).collect::<String>();
    let is_seed = migration.is_seed;

    view! {
        <div class="migration-card" role="listitem">
            // ── Collapsed row (always visible) ──────────────────────────
            <button
                class="migration-card-row"
                aria-expanded=move || open.get().to_string()
                on:click=move |_| open.update(|v| *v = !*v)
            >
                <span class="migration-order-badge" aria-hidden="true">
                    {order_num}
                </span>
                <span class="migration-row-file">{file.get_value()}</span>
                {is_seed.then(|| view! {
                    <span
                        style="display:inline-flex;align-items:center;padding:1px 7px;border-radius:12px;font-size:0.65rem;font-weight:600;letter-spacing:0.05em;background:rgba(16,185,129,0.12);color:#10b981;border:1px solid rgba(16,185,129,0.35);margin-left:4px;"
                        aria-label="seed migration"
                    >"SEED"</span>
                })}
                <span class="migration-row-why">{why.get_value()}</span>
                <span class="migration-row-chips">
                    <span class="checksum-chip">{checksum_short}"…"</span>
                    <span class="meta-chip">{created.get_value()}</span>
                    <span class="meta-chip">"by: "{author.get_value()}</span>
                </span>
                <span
                    class=move || {
                        if open.get() { "migration-chevron open" } else { "migration-chevron" }
                    }
                    aria-hidden="true"
                >
                    "\u{276F}"
                </span>
            </button>

            // ── Expanded body ────────────────────────────────────────────
            {move || open.get().then(|| view! {
                <div class="migration-card-body">
                    // Expanded detail header: why + meta
                    <div class="migration-card-detail-header">
                        <p class="migration-card-detail-why">{why.get_value()}</p>
                        <div class="lineage-meta">
                            <span class="checksum-chip">
                                "sha256: "
                                {migration.checksum.chars().take(14).collect::<String>()}
                                "\u{2026}"
                            </span>
                            <span class="meta-chip">{created.get_value()}</span>
                            <span class="meta-chip">"by: "{author.get_value()}</span>
                        </div>
                    </div>
                    // UP / DOWN tab bar
                    <div class="lineage-tabs-list" role="tablist" aria-label="SQL sections">
                        <button
                            role="tab"
                            aria-selected=move || (tab.get() == "up").to_string()
                            class=move || {
                                if tab.get() == "up" { "lineage-tab-btn active-up" }
                                else { "lineage-tab-btn" }
                            }
                            on:click=move |_| tab.set("up".to_string())
                        >
                            "\u{25B2} UP"
                        </button>
                        <button
                            role="tab"
                            aria-selected=move || (tab.get() == "down").to_string()
                            class=move || {
                                if tab.get() == "down" { "lineage-tab-btn active-down" }
                                else { "lineage-tab-btn" }
                            }
                            on:click=move |_| tab.set("down".to_string())
                        >
                            "\u{25BC} DOWN"
                        </button>
                    </div>
                    // SQL content
                    <div class="lineage-code-area">
                        {move || {
                            if tab.get() == "up" {
                                view! {
                                    <CodeBlock
                                        code=up_sql.get_value()
                                        language="sql".to_string()
                                        filename=file.get_value()
                                    />
                                }.into_any()
                            } else {
                                let down = down_sql.get_value();
                                if down.trim().is_empty() {
                                    view! {
                                        <p class="text-sm italic" style="color: hsl(var(--muted-foreground)); padding: 0.5rem 0;">
                                            "No DOWN section defined."
                                        </p>
                                    }.into_any()
                                } else {
                                    view! {
                                        <CodeBlock
                                            code=down
                                            language="sql".to_string()
                                            filename=file.get_value()
                                        />
                                    }.into_any()
                                }
                            }
                        }}
                    </div>
                </div>
            })}
        </div>
    }
}

// ── One version section (header + its migration cards) ───────────────────────
#[component]
fn VersionSection(block: VersionBlock) -> impl IntoView {
    let count = block.migrations.len();
    let version = block.version;

    view! {
        <div class="version-section">
            <div class="version-header">
                <span class="version-badge">
                    "V"{version}
                </span>
                <div class="version-meta-row">
                    <span class="version-stats">
                        {count}" migration"{if count == 1 { "" } else { "s" }}" \u{00B7} applied in order"
                    </span>
                </div>
            </div>
            <div role="list" aria-label={format!("Version {} migrations", version)}>
                {block.migrations.into_iter().map(|m| {
                    let order_num = m.order_index + 1;
                    view! { <MigrationCard order_num=order_num migration=m /> }
                }).collect_view()}
            </div>
        </div>
    }
}

// ── Timeline view (version-grouped) ─────────────────────────────────────────
#[component]
fn TimelineView(versions: Vec<VersionBlock>, setup_files: Vec<String>) -> impl IntoView {
    view! {
        <div>
            <div class="landing-container">
                // Legend
                <div class="timeline-legend" role="note">
                    <span class="tl-forward">"\u{2193} Apply"</span>
                    ": top-to-bottom within and across versions.\u{2002}"
                    <span class="tl-rollback">"\u{21BA} Rollback"</span>
                    ": exact reverse of the apply order \u{2014} guarantees FK-safe teardown."
                </div>

                // Version sections
                {versions.into_iter().map(|block| {
                    view! { <VersionSection block=block /> }
                }).collect_view()}
            </div>

            // Setup files
            <div class="landing-container mt-12">
                <div class="setup-callout">
                    <p class="setup-callout-label">"00_setup \u{2014} Untracked bootstrap files"</p>
                    <p class="text-xs mb-3" style="color: hsl(var(--muted-foreground)); line-height: 1.55;">
                        "These run before every "
                        <code class="font-mono" style="font-size: 0.9em;">"up()"</code>
                        " call \u{2014} idempotent SQL for schema creation, extensions, and grants. "
                        "Never recorded in the migrations tracking table."
                    </p>
                    {setup_files.into_iter().map(|f| {
                        view! {
                            <div class="setup-file-row">
                                <span style="color: hsl(var(--primary)); font-size: 0.7rem;" aria-hidden="true">"\u{25CF}"</span>
                                <span class="setup-filename">{f}</span>
                                <span class="meta-chip">"setup"</span>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>
        </div>
    }
}

// ── Schema (ERD) view ────────────────────────────────────────────────────────
#[component]
fn SchemaView() -> impl IntoView {
    let (tables, relations) = load_schema();
    view! {
        <div class="landing-container">
            <div class="schema-canvas-frame">
                <SchemaDiagram tables=tables relations=relations />
            </div>
            <div class="schema-legend" aria-label="ERD legend">
                <span class="schema-legend-item">
                    <span class="schema-legend-key" aria-hidden="true">"\u{1F511}"</span>
                    "Primary key"
                </span>
                <span class="schema-legend-item">
                    <span class="schema-legend-key" aria-hidden="true">"\u{1F517}"</span>
                    "Foreign key \u{2014} lines show the joins"
                </span>
            </div>
        </div>
    }
}

/// If `s` looks like a UUID (8-4-4-4-12 hex), return first 8 chars + … + last 4.
/// Otherwise return the original string unchanged.
fn short_val(s: &str) -> String {
    // UUID pattern: 8-4-4-4-12 hex digits separated by hyphens
    let is_uuid = s.len() == 36
        && s.as_bytes()[8] == b'-'
        && s.as_bytes()[13] == b'-'
        && s.as_bytes()[18] == b'-'
        && s.as_bytes()[23] == b'-'
        && s.chars().enumerate().all(|(i, c)| {
            i == 8 || i == 13 || i == 18 || i == 23 || c.is_ascii_hexdigit()
        });
    if is_uuid {
        format!("{}\u{2026}{}", &s[..8], &s[32..])
    } else {
        s.to_owned()
    }
}

// ── Data view (seed rows as tables) ──────────────────────────────────────────
#[component]
fn DataView() -> impl IntoView {
    let (tables, _) = load_schema();
    let seed_data = load_seed_data();

    view! {
        <div class="landing-container">
            {tables.into_iter().map(|t| {
                let table_name = t.name.clone();
                let seed: Option<SeedTable> = seed_data.iter().find(|s| s.table == table_name).cloned();
                let display_name = format!("vault.{table_name}");

                let row_count = seed.as_ref().map(|s| s.rows.len()).unwrap_or(0);
                let row_label = if row_count == 0 {
                    "\u{00B7} no seed data".to_string()
                } else if row_count == 1 {
                    "\u{00B7} 1 row".to_string()
                } else {
                    format!("\u{00B7} {row_count} rows")
                };

                let (columns, rows, has_data) = match seed {
                    Some(s) => {
                        let cols = s.columns.clone();
                        let rows = s.rows.clone();
                        (cols, rows, true)
                    }
                    None => {
                        let cols: Vec<String> = t.columns.iter().map(|c| c.name.clone()).collect();
                        (cols, vec![], false)
                    }
                };

                let header_cols = columns.clone();
                view! {
                    <div class="seed-table-card">
                        <div class="seed-table-title">
                            {display_name}
                            <span class="seed-row-count">{row_label}</span>
                        </div>
                        <div class="seed-table-scroll">
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        {header_cols.into_iter().map(|col| {
                                            view! { <TableHead class="seed-table-th".to_string()>{col}</TableHead> }
                                        }).collect_view()}
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {if has_data {
                                        rows.into_iter().map(|row| {
                                            let row_cols = columns.clone();
                                            view! {
                                                <TableRow>
                                                    {row.into_iter().enumerate().map(|(i, cell)| {
                                                        let is_id_col = row_cols.get(i)
                                                            .map(|c| c == "id" || c.ends_with("_id"))
                                                            .unwrap_or(false);
                                                        let display = short_val(&cell);
                                                        let is_truncated = display != cell;
                                                        let full = cell.clone();
                                                        view! {
                                                            <TableCell>
                                                                {if is_id_col {
                                                                    view! {
                                                                        <span
                                                                            style="font-family: var(--font-mono, monospace); font-size: 0.8em; color: hsl(var(--muted-foreground));"
                                                                            title=if is_truncated { full } else { String::new() }
                                                                        >{display}</span>
                                                                    }.into_any()
                                                                } else {
                                                                    view! { <span title=if is_truncated { full } else { String::new() }>{display}</span> }.into_any()
                                                                }}
                                                            </TableCell>
                                                        }
                                                    }).collect_view()}
                                                </TableRow>
                                            }
                                        }).collect_view().into_any()
                                    } else {
                                        view! {
                                            <TableRow>
                                                <TableCell>
                                                    <span style="color: hsl(var(--muted-foreground)); font-style: italic; font-size: 0.875rem;">
                                                        "No seed data"
                                                    </span>
                                                </TableCell>
                                            </TableRow>
                                        }.into_any()
                                    }}
                                </TableBody>
                            </Table>
                        </div>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

// ── Main Explorer Page ───────────────────────────────────────────────────────
#[component]
pub fn ExplorerPage() -> impl IntoView {
    let data = load_migrations();
    let versions = data.versions.clone();
    let setup_files = data.setup_files.clone();

    let view_mode: RwSignal<String> = RwSignal::new("Timeline".to_string());

    view! {
        <div class="page-atmosphere pb-20">
            // ── HERO (reactive heading) ───────────────────────────────────
            <div class="landing-container pt-14 pb-8">
                <span class="explorer-hero-eyebrow">"Migration Explorer"</span>
                // heading and subtitle are reactive to view_mode
                {move || {
                    if view_mode.get() == "Timeline" {
                        view! {
                            <div>
                                <h1 class="explorer-hero-title">
                                    "Apply order. Rollback order.\u{a0}"
                                    <span style="color: hsl(var(--forward));">"Exact"</span>
                                    " reverse."
                                </h1>
                                <p class="explorer-hero-sub mt-3">
                                    "soma-schema applies migrations top-to-bottom as listed in "
                                    <code class="font-mono" style="font-size: 0.85em; color: hsl(var(--foreground) / 0.8);">"migration-order.yaml"</code>
                                    " and rolls back in exact reverse manifest order \u{2014} not filename sort. "
                                    "Expand any migration to inspect its UP and DOWN SQL."
                                </p>
                            </div>
                        }.into_any()
                    } else if view_mode.get() == "Schema" {
                        view! {
                            <div>
                                <h1 class="explorer-hero-title">
                                    "The schema your migrations build"
                                </h1>
                                <p class="explorer-hero-sub mt-3">
                                    "Every table and foreign key these migrations create. "
                                    "Drag tables to rearrange, scroll to zoom, hover a table to trace its relationships."
                                </p>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div>
                                <h1 class="explorer-hero-title">"Seed data"</h1>
                                <p class="explorer-hero-sub mt-3">
                                    "The rows your seed migrations insert \u{2014} your database, in table form."
                                </p>
                            </div>
                        }.into_any()
                    }
                }}

                // ── View Toggle ───────────────────────────────────────────
                <div class="view-toggle mt-6" role="group" aria-label="View mode">
                    <button
                        class=move || {
                            if view_mode.get() == "Timeline" {
                                "view-toggle-btn view-toggle-btn-active"
                            } else {
                                "view-toggle-btn"
                            }
                        }
                        aria-pressed=move || (view_mode.get() == "Timeline").to_string()
                        on:click=move |_| view_mode.set("Timeline".to_string())
                    >
                        "\u{25A6} Timeline"
                    </button>
                    <button
                        class=move || {
                            if view_mode.get() == "Schema" {
                                "view-toggle-btn view-toggle-btn-active"
                            } else {
                                "view-toggle-btn"
                            }
                        }
                        aria-pressed=move || (view_mode.get() == "Schema").to_string()
                        on:click=move |_| view_mode.set("Schema".to_string())
                    >
                        "\u{229E} Schema"
                    </button>
                    <button
                        class=move || {
                            if view_mode.get() == "Data" {
                                "view-toggle-btn view-toggle-btn-active"
                            } else {
                                "view-toggle-btn"
                            }
                        }
                        aria-pressed=move || (view_mode.get() == "Data").to_string()
                        on:click=move |_| view_mode.set("Data".to_string())
                    >
                        "\u{229F} Data"
                    </button>
                </div>
            </div>

            // ── CONDITIONAL BODY ─────────────────────────────────────────
            {move || {
                if view_mode.get() == "Schema" {
                    view! { <SchemaView /> }.into_any()
                } else if view_mode.get() == "Data" {
                    view! { <DataView /> }.into_any()
                } else {
                    let versions = versions.clone();
                    let setup_files = setup_files.clone();
                    view! { <TimelineView versions=versions setup_files=setup_files /> }.into_any()
                }
            }}
        </div>
    }
}
