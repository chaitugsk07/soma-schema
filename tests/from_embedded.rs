/// Tests for Migrator::from_embedded — public API surface only.
///
/// No Postgres required. Verifies that from_embedded + an in-memory driver
/// runs the discovery/apply pipeline correctly.
///
/// Checksum byte-identity between from_embedded and from_root is tested in
/// src/migrator.rs (#[cfg(test)]) where the private `root` field is accessible.
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use soma_schema::driver::{AppliedMigration, LockGuard, MigrationDriver};
use soma_schema::error::Result;
use soma_schema::include_dir::include_dir;
use soma_schema::migration::Migration;
use soma_schema::Migrator;

// Embed the fixture migrations directory at compile time.
static FIXTURE_DIR: soma_schema::include_dir::Dir =
    include_dir!("$CARGO_MANIFEST_DIR/tests/fixtures/embedded-migrations");

// ---------------------------------------------------------------------------
// Minimal in-memory driver (mirrors in_memory_driver.rs pattern)
// ---------------------------------------------------------------------------

struct InMemoryDriver {
    rows: Mutex<Vec<AppliedMigration>>,
    apply_order: Mutex<Vec<String>>,
}

impl InMemoryDriver {
    fn new() -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            apply_order: Mutex::new(Vec::new()),
        }
    }
}

struct NoopLock;
impl LockGuard for NoopLock {}

#[async_trait]
impl MigrationDriver for InMemoryDriver {
    async fn acquire_lock(&self) -> Result<Box<dyn LockGuard>> {
        Ok(Box::new(NoopLock))
    }

    async fn run_setup_sql(&self, _name: &str, _sql: &str) -> Result<()> {
        Ok(())
    }

    async fn ensure_tracking_table(&self) -> Result<()> {
        Ok(())
    }

    async fn applied(&self) -> Result<Vec<AppliedMigration>> {
        let rows = self.rows.lock().unwrap();
        let out = rows
            .iter()
            .map(|r| {
                AppliedMigration::new(
                    r.version,
                    r.file.clone(),
                    r.name.clone(),
                    r.checksum.clone(),
                    r.description.clone(),
                    r.batch,
                    r.applied_at,
                    r.applied_by.clone(),
                    r.execution_ms,
                )
            })
            .collect();
        Ok(out)
    }

    async fn apply(&self, migration: &Migration, _up_sql: &str, batch: i32) -> Result<()> {
        let row = AppliedMigration::new(
            migration.version,
            migration.file.clone(),
            migration.name.clone(),
            migration.checksum.clone(),
            None,
            batch,
            Utc::now(),
            "in_memory".to_owned(),
            None,
        );
        self.rows.lock().unwrap().push(row);
        self.apply_order
            .lock()
            .unwrap()
            .push(migration.file.clone());
        Ok(())
    }

    async fn revert(&self, applied: &AppliedMigration, _down_sql: &str) -> Result<()> {
        let mut rows = self.rows.lock().unwrap();
        if let Some(pos) = rows
            .iter()
            .position(|r| r.version == applied.version && r.file == applied.file)
        {
            rows.remove(pos);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// from_embedded succeeds and up() via an in-memory driver applies the fixture migration.
#[tokio::test]
async fn test_from_embedded_up_applies_migrations() {
    let migrator = Migrator::from_embedded(&FIXTURE_DIR).expect("from_embedded should succeed");
    let driver = InMemoryDriver::new();

    migrator
        .up(&driver)
        .await
        .expect("up() via from_embedded should succeed");

    let order = driver.apply_order.lock().unwrap().clone();
    assert_eq!(order.len(), 1, "one migration in the fixture");
    assert_eq!(order[0], "20260101_01_init.sql");
}

/// from_embedded is idempotent: a second up() applies nothing new.
#[tokio::test]
async fn test_from_embedded_up_is_idempotent() {
    let migrator = Migrator::from_embedded(&FIXTURE_DIR).expect("from_embedded should succeed");
    let driver = InMemoryDriver::new();

    migrator.up(&driver).await.unwrap();
    migrator.up(&driver).await.unwrap();

    let order = driver.apply_order.lock().unwrap().clone();
    assert_eq!(order.len(), 1, "second up() should apply nothing new");
}
