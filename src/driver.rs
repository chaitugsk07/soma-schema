use async_trait::async_trait;

use crate::error::Result;
use crate::migration::Migration;

/// A deployed migration row from the tracking table.
#[derive(Debug)]
pub struct AppliedMigration {
    pub version: u32,
    pub file: String,
    pub name: String,
    pub checksum: String,
    pub description: Option<String>,
    pub batch: i32,
    pub applied_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
    pub applied_by: String,
    pub execution_ms: Option<i32>,
}

/// A lock guard. Releases the advisory lock on Drop.
/// The concrete implementation keeps a dedicated connection alive.
pub trait LockGuard: Send + Sync {}

/// Object-safe driver trait. Implement for each database backend.
#[async_trait]
pub trait MigrationDriver: Send + Sync {
    /// Acquire an advisory lock. The lock is held until the returned guard is dropped.
    async fn acquire_lock(&self) -> Result<Box<dyn LockGuard>>;

    /// Execute one 00_setup file's SQL in a single transaction (untracked).
    async fn run_setup_sql(&self, name: &str, sql: &str) -> Result<()>;

    /// Ensure the deployment tracking table (and schema, if configured) exist.
    async fn ensure_tracking_table(&self) -> Result<()>;

    /// Return all applied migrations, ordered by (version ASC, file ASC).
    async fn applied(&self) -> Result<Vec<AppliedMigration>>;

    /// Apply a migration: run UP SQL AND insert the tracking row in ONE transaction.
    /// The `up_sql` is passed in rather than re-reading the file in the driver.
    async fn apply(&self, migration: &Migration, up_sql: &str, batch: i32) -> Result<()>;

    /// Revert an applied migration: run `down_sql` AND delete the tracking row in ONE transaction.
    async fn revert(&self, applied: &AppliedMigration, down_sql: &str) -> Result<()>;
}
