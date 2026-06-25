use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("migration-order.yaml not found at the migrations root")]
    ManifestMissing,

    #[error("failed to parse migration-order.yaml: {0}")]
    ManifestParse(String),

    #[error("unsupported manifest_version {found}; this tool only supports version 1")]
    UnsupportedManifestVersion { found: u32 },

    #[error(
        "invalid migration filename (must be a bare .sql name, no path separators, no '..'): {0}"
    )]
    InvalidFileName(String),

    #[error("duplicate entry in manifest: version {version}, file {file}")]
    DuplicateEntry { version: u32, file: String },

    #[error("on-disk migration not listed in manifest: {path}")]
    OrphanMigration { path: String },

    #[error("manifest entry not found on disk: version {version}, file {file}")]
    MissingFile { version: u32, file: String },

    #[error("checksum drift for version {version}, file {file}: applied checksum differs from file on disk")]
    ChecksumDrift { version: u32, file: String },

    #[error("no DOWN section in version {version}, file {file}")]
    MissingDown { version: u32, file: String },

    #[error("identifier contains invalid characters (only [A-Za-z0-9_] allowed): {0}")]
    InvalidIdentifier(String),

    #[error("setup file {file} failed: {source}")]
    SetupFailed {
        file: String,
        #[source]
        source: sqlx::Error,
    },

    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// The pool must have at least 2 connections: one is reserved for the advisory lock.
    #[error(
        "pool too small: max_connections must be >= 2 (one is reserved for the advisory lock)"
    )]
    PoolTooSmall,

    /// An applied migration (present in the tracking table) is no longer on disk or in
    /// the manifest. Silently skipping it would bypass the integrity guard — surface it
    /// as an explicit error so operators notice the tampered or deleted migration file.
    #[error("migration version {version}, file {file} is recorded as applied but is missing from the manifest and disk")]
    AppliedButMissing { version: u32, file: String },

    #[error("explorer build failed: {0}")]
    Explorer(String),
}

pub type Result<T> = std::result::Result<T, Error>;
