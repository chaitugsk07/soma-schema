#[cfg(feature = "cli")]
mod cli {
    use clap::{Parser, Subcommand};
    use sqlx::postgres::PgPoolOptions;
    use std::path::PathBuf;

    use soma_schema::{Migrator, PostgresConfig, PostgresDriver};

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

    #[derive(Subcommand)]
    enum Command {
        /// Scaffold a new migrations root directory with a starter layout.
        Init {
            /// Directory to initialise. Defaults to --migrations value.
            dir: Option<PathBuf>,
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

        if let Command::Init { dir } = &cli.command {
            let root = dir.as_ref().unwrap_or(&cli.migrations);
            Migrator::scaffold(root)?;
            println!("Initialised migrations root at: {}", root.display());
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
            Command::Init { .. } | Command::Explorer { .. } => unreachable!(),
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
