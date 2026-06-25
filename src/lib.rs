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
pub use error::{Error, Result};
pub use migration::{Migration, SetupFile};
pub use migrator::Migrator;
pub use postgres::{PostgresConfig, PostgresDriver};

#[cfg(feature = "explorer")]
pub use explorer::{build_json, render_html};
