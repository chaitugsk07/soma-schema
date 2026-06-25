//! # soma-schema
//!
//! A standalone Postgres migration tool — plain SQL files with `UP`/`DOWN` sections,
//! manifest-driven ordering, SHA-256 checksum drift detection, and advisory-lock safety
//! to prevent concurrent `up`/`down` runs.
//!
//! **Differentiators:** (1) manifest-first ordering (not filename-lexical) for FK-safe
//! rollback; (2) checksum on the full raw file (UP + DOWN) so even editing the DOWN
//! section of an applied migration is caught; (3) apply + track in a single transaction
//! (no split state on crash); (4) advisory lock scoped to the whole `up`/`down` call
//! (not per-migration).
//!
//! ## Quick start
//!
//! ```no_run
//! # use soma_schema::{Migrator, PostgresConfig, PostgresDriver};
//! # async fn run() -> soma_schema::Result<()> {
//! let pool = sqlx::PgPool::connect("postgres://localhost/mydb").await?;
//! let config = PostgresConfig {
//!     schema: Some("app".into()),
//!     ..Default::default()
//! };
//! let driver = PostgresDriver::new(pool, config)?;
//! let migrator = Migrator::from_root("migrations");
//! migrator.up(&driver).await?;
//! # Ok(())
//! # }
//! ```

pub mod discovery;
pub mod driver;
pub mod error;
pub mod manifest;
pub mod migration;
pub mod migrator;
pub mod postgres;

#[cfg(feature = "explorer")]
pub mod explorer;

pub use discovery::discover;
pub use driver::{AppliedMigration, LockGuard, MigrationDriver};
pub use error::{Error, Result};
pub use migration::{Migration, SetupFile};
pub use migrator::{MigrationStatus, Migrator, PendingMigration};
pub use postgres::{PostgresConfig, PostgresDriver};

#[cfg(feature = "explorer")]
pub use explorer::{build_json, render_html};
