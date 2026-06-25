#[cfg(feature = "cli")]
mod cli {
    use clap::{Parser, Subcommand};
    use sqlx::postgres::PgPoolOptions;
    use std::path::PathBuf;

    use soma_schema::{Migrator, PostgresConfig, PostgresDriver, RulesTarget};

    #[derive(Parser)]
    #[command(
        name = "soma-schema",
        about = "Plain-SQL database migration tool — UP/DOWN, version tracking, advisory-lock safety",
        version
    )]
    struct Cli {
        /// Path to the migrations root directory (containing migration-order.yaml).
        #[arg(long, default_value = "migrations")]
        migrations: PathBuf,

        /// Target database URL. Falls back to DATABASE_URL env var. Not required for
        /// the `explorer` subcommand.
        #[arg(long, env = "DATABASE_URL", required = false)]
        database_url: Option<String>,

        /// Override the target schema (default: connection's search_path).
        #[arg(long)]
        schema: Option<String>,

        /// Override the tracking table name (default: 00_schema_migrations).
        #[arg(long)]
        table: Option<String>,

        #[command(subcommand)]
        command: Command,
    }

    #[derive(clap::ValueEnum, Clone, Debug)]
    enum ExplorerFormat {
        Html,
        Json,
    }

    /// Which agent-rules file(s) `init` should write into the current directory.
    #[derive(clap::ValueEnum, Clone, Debug, Default)]
    enum RulesMode {
        /// Write `AGENTS.md` (works with Claude Code, OpenAI Codex, Cursor, Zed, …)
        #[default]
        Agents,
        /// Write `CLAUDE.md`
        Claude,
        /// Write `.cursor/rules/soma-schema.mdc`
        Cursor,
        /// Write `.windsurf/rules/soma-schema.md`
        Windsurf,
        /// Write all of the above
        All,
        /// Skip rules writing
        None,
    }

    impl From<&RulesMode> for RulesTarget {
        fn from(m: &RulesMode) -> Self {
            match m {
                RulesMode::Agents => RulesTarget::Agents,
                RulesMode::Claude => RulesTarget::Claude,
                RulesMode::Cursor => RulesTarget::Cursor,
                RulesMode::Windsurf => RulesTarget::Windsurf,
                RulesMode::All => RulesTarget::All,
                RulesMode::None => RulesTarget::None,
            }
        }
    }

    #[derive(Subcommand)]
    enum Command {
        /// Scaffold a new migrations root directory with a starter layout,
        /// then write agent-rules into the current directory so any AI agent
        /// knows the soma-schema conventions.
        ///
        /// One command delivers: migrations/ directory + runnable example +
        /// AGENTS.md with the canonical rules. Then: set DATABASE_URL and run
        /// `soma-schema up`.
        Init {
            /// Directory to initialise (default: "migrations").
            dir: Option<PathBuf>,
            /// Which agent-rules file(s) to write into CWD (default: agents → AGENTS.md).
            /// If the target file already exists, the rules section is appended
            /// idempotently — existing content is never clobbered.
            #[arg(long, default_value = "agents")]
            rules: RulesMode,
            /// Also install the /soma-schema Claude skill to
            /// ~/.claude/skills/soma-schema/SKILL.md.
            #[arg(long)]
            skill: bool,
            /// Open the visual explorer after scaffolding (requires the `explorer`
            /// feature; prints a hint when the feature is disabled).
            #[arg(long)]
            explore: bool,
        },
        /// Apply all pending migrations.
        Up,
        /// Revert the last N applied migrations (default: 1).
        Down {
            #[arg(long, default_value_t = 1)]
            steps: usize,
        },
        /// Show applied and pending migrations.
        Status,
        /// Open a visual schema + migration explorer in your browser (no database needed).
        Explorer {
            /// Output format: html or json
            #[arg(long, default_value = "html")]
            format: ExplorerFormat,
            /// Output path (defaults to a temp file for html, stdout for json)
            #[arg(long)]
            out: Option<PathBuf>,
            /// Don't open the browser after writing the file (html only)
            #[arg(long)]
            no_open: bool,
        },
    }

    pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
        let cli = Cli::parse();

        if let Command::Init {
            dir,
            rules,
            skill,
            explore,
        } = &cli.command
        {
            let root = dir.as_ref().unwrap_or(&cli.migrations);
            Migrator::scaffold(root)?;
            println!("Migrations directory:  {}", root.display());

            // Write agent-rules into CWD.
            let target = RulesTarget::from(rules);
            let cwd = std::env::current_dir()?;
            let rule_msgs = soma_schema::write_rules(&cwd, &target)?;
            for msg in &rule_msgs {
                println!("Agent rules:           {msg}");
            }

            // Optionally install the Claude skill.
            if *skill {
                let skill_msg = soma_schema::install_skill()?;
                println!("Claude skill:          {skill_msg}");
            }

            // Optionally open the explorer.
            if *explore {
                #[cfg(feature = "explorer")]
                {
                    let html = soma_schema::explorer::render_html(root)?;
                    let path = std::env::temp_dir().join("soma-schema-explorer.html");
                    std::fs::write(&path, &html)?;
                    eprintln!("Wrote explorer to {}", path.display());
                    let result = open_in_browser(&path);
                    if result.is_err() {
                        eprintln!("Open this file in your browser: {}", path.display());
                    }
                }
                #[cfg(not(feature = "explorer"))]
                {
                    println!(
                        "Tip: run `soma-schema explorer` once you have installed the binary \
                         with the `explorer` feature (default)."
                    );
                }
            }

            println!();
            println!("Next steps:");
            println!("  1. Set DATABASE_URL (or pass --database-url)");
            println!("  2. soma-schema up          — apply the example migration");
            println!("  3. soma-schema status       — verify");
            println!("  4. soma-schema explorer     — open the visual UI");

            return Ok(());
        }

        if let Command::Explorer {
            format,
            out,
            no_open,
        } = &cli.command
        {
            let root = &cli.migrations;
            match format {
                ExplorerFormat::Json => {
                    let json = soma_schema::explorer::build_json(root)?;
                    if let Some(path) = out {
                        std::fs::write(path, &json)?;
                        eprintln!("Wrote JSON to {}", path.display());
                    } else {
                        print!("{json}");
                    }
                }
                ExplorerFormat::Html => {
                    let html = soma_schema::explorer::render_html(root)?;
                    let path = out
                        .clone()
                        .unwrap_or_else(|| std::env::temp_dir().join("soma-schema-explorer.html"));
                    std::fs::write(&path, &html)?;
                    eprintln!("Wrote explorer to {}", path.display());
                    if !no_open {
                        // ponytail: best-effort open; doesn't special-case WSL or headless environments
                        let result = open_in_browser(&path);
                        if result.is_err() {
                            eprintln!("Open this file in your browser: {}", path.display());
                        }
                    }
                }
            }
            return Ok(());
        }

        let database_url = cli
            .database_url
            .ok_or("database URL required (use --database-url or set DATABASE_URL env var)")?;

        let mut pg_config = PostgresConfig::default();
        if let Some(s) = &cli.schema {
            pg_config.schema = Some(s.clone());
        }
        if let Some(t) = &cli.table {
            pg_config.table = t.clone();
        }

        // 2 connections: one held for the advisory lock, one for migration work.
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await?;

        let driver = PostgresDriver::new(pool, pg_config)?;
        let migrator = Migrator::from_root(&cli.migrations);

        match &cli.command {
            Command::Init { .. } | Command::Explorer { .. } => {
                unreachable!("handled above")
            }
            Command::Up => {
                migrator.up(&driver).await?;
                println!("All pending migrations applied.");
            }
            Command::Down { steps } => {
                migrator.down(&driver, *steps).await?;
                println!("Reverted {} migration(s).", steps);
            }
            Command::Status => {
                let status = migrator.status(&driver).await?;
                println!("Applied ({}):", status.applied.len());
                for a in &status.applied {
                    println!(
                        "  [v{}] {} (batch={}, applied_at={}, by={})",
                        a.version, a.file, a.batch, a.applied_at, a.applied_by
                    );
                }
                println!("Pending ({}):", status.pending.len());
                for p in &status.pending {
                    let created = p.created.as_deref().unwrap_or("-");
                    let author = p.author.as_deref().unwrap_or("-");
                    let desc = p.version_description.as_deref().unwrap_or("-");
                    println!(
                        "  [v{} \"{desc}\"] {} (created={}, author={})",
                        p.version, p.file, created, author
                    );
                }
                if !status.drift_errors.is_empty() {
                    eprintln!(
                        "\n⚠ drift detected ({} issue{}):",
                        status.drift_errors.len(),
                        if status.drift_errors.len() != 1 {
                            "s"
                        } else {
                            ""
                        }
                    );
                    for err in &status.drift_errors {
                        eprintln!("  - {err}");
                    }
                }
            }
        }

        Ok(())
    }

    fn open_in_browser(path: &std::path::Path) -> std::io::Result<()> {
        let path_str = path.to_string_lossy();
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(path_str.as_ref())
                .status()?;
        }
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(path_str.as_ref())
                .status()?;
        }
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/C", "start", "", path_str.as_ref()])
                .status()?;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    #[cfg(feature = "cli")]
    {
        if let Err(e) = cli::run().await {
            eprintln!("Error: {e:#}");
            std::process::exit(1);
        }
    }
    #[cfg(not(feature = "cli"))]
    {
        eprintln!("CLI feature not enabled.");
        std::process::exit(1);
    }
}
