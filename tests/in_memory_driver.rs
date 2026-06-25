/// In-memory driver tests — backend-agnostic trait coverage.
///
/// This file implements `InMemoryDriver` (soma_schema::MigrationDriver) using only
/// in-process state. No Postgres, no TEST_DATABASE_URL needed.
///
/// Now that `AppliedMigration::new` exists, `InMemoryDriver` stores real
/// `AppliedMigration` rows and removes them on revert. This makes it a full
/// end-to-end in-memory implementation of the trait, exercising idempotency,
/// down ordering, and status accuracy.
use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use tempfile::TempDir;

use soma_schema::driver::{AppliedMigration, LockGuard, MigrationDriver};
use soma_schema::error::Result;
use soma_schema::migration::Migration;
use soma_schema::{MigrationStatus, Migrator};

// ---------------------------------------------------------------------------
// InMemoryDriver
// ---------------------------------------------------------------------------

/// Full in-memory implementation of `MigrationDriver`.
///
/// Stores applied rows so `applied()` returns accurate data, enabling
/// idempotency on `up()` and correct reverse-order revert on `down()`.
struct InMemoryDriver {
    /// Applied rows in insertion order.
    rows: Mutex<Vec<AppliedMigration>>,
    /// (version, file) pairs in apply order — for order assertions.
    apply_order: Mutex<Vec<(u32, String)>>,
    /// (version, file) pairs in revert order — for order assertions.
    revert_order: Mutex<Vec<(u32, String)>>,
}

impl InMemoryDriver {
    fn new() -> Self {
        Self {
            rows: Mutex::new(Vec::new()),
            apply_order: Mutex::new(Vec::new()),
            revert_order: Mutex::new(Vec::new()),
        }
    }

    fn apply_order(&self) -> Vec<(u32, String)> {
        self.apply_order.lock().unwrap().clone()
    }

    fn revert_order(&self) -> Vec<(u32, String)> {
        self.revert_order.lock().unwrap().clone()
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
        // Return cloned rows. We can't derive Clone on AppliedMigration (it's
        // #[non_exhaustive] and owned by the library), so we rebuild via ::new().
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
            .push((migration.version, migration.file.clone()));
        Ok(())
    }

    async fn revert(&self, applied: &AppliedMigration, _down_sql: &str) -> Result<()> {
        let mut rows = self.rows.lock().unwrap();
        // Remove the first row matching (version, file).
        if let Some(pos) = rows
            .iter()
            .position(|r| r.version == applied.version && r.file == applied.file)
        {
            rows.remove(pos);
        }
        self.revert_order
            .lock()
            .unwrap()
            .push((applied.version, applied.file.clone()));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

fn sql_with_down(table: &str) -> String {
    format!("CREATE TABLE {table} (id INT);\n-- DOWN ==\nDROP TABLE IF EXISTS {table};")
}

struct Fixture {
    _dir: TempDir,
    pub root: PathBuf,
}

impl Fixture {
    fn build(versions: &[(u32, Vec<(&str, &str)>)]) -> Self {
        let dir = TempDir::new().unwrap();
        let root = dir.path().to_path_buf();

        let mut yaml = String::from("manifest_version: 1\nversions:\n");
        for (v, files) in versions {
            yaml.push_str(&format!("  - version: {v}\n    migrations:\n"));
            std::fs::create_dir_all(root.join("01_migrated").join(v.to_string())).unwrap();
            for (fname, content) in files {
                yaml.push_str(&format!("      - file: \"{fname}\"\n"));
                std::fs::write(
                    root.join("01_migrated").join(v.to_string()).join(fname),
                    content,
                )
                .unwrap();
            }
        }
        std::fs::write(root.join("migration-order.yaml"), yaml).unwrap();
        Fixture { _dir: dir, root }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// up() applies migrations in manifest order (not filename sort order).
#[tokio::test]
async fn test_in_memory_up_applies_in_manifest_order() {
    // Filenames sort b < z alphabetically, but manifest lists z first.
    let f = Fixture::build(&[(
        1,
        vec![
            ("z_first.sql", &sql_with_down("z_tbl")),
            ("b_second.sql", &sql_with_down("b_tbl")),
        ],
    )]);

    let driver = InMemoryDriver::new();
    let migrator = Migrator::from_root(&f.root);
    migrator.up(&driver).await.expect("up() should succeed");

    let order = driver.apply_order();
    assert_eq!(order.len(), 2, "both migrations must be applied");
    assert_eq!(
        order[0].1, "z_first.sql",
        "z_first must come first (manifest order)"
    );
    assert_eq!(order[1].1, "b_second.sql", "b_second must come second");
}

/// up() is idempotent: a second call applies nothing (driver returns real rows).
#[tokio::test]
async fn test_in_memory_up_is_idempotent() {
    let f = Fixture::build(&[(
        1,
        vec![
            ("a.sql", &sql_with_down("ta")),
            ("b.sql", &sql_with_down("tb")),
        ],
    )]);

    let driver = InMemoryDriver::new();
    let migrator = Migrator::from_root(&f.root);

    migrator
        .up(&driver)
        .await
        .expect("first up() should succeed");
    assert_eq!(driver.apply_order().len(), 2, "first up applies 2");

    migrator
        .up(&driver)
        .await
        .expect("second up() should succeed");
    assert_eq!(
        driver.apply_order().len(),
        2,
        "second up applies nothing new"
    );
}

/// up() across multiple version folders respects numeric (not lexicographic) order.
#[tokio::test]
async fn test_in_memory_up_numeric_version_order() {
    let f = Fixture::build(&[
        (1, vec![("v1.sql", &sql_with_down("t1"))]),
        (2, vec![("v2.sql", &sql_with_down("t2"))]),
        (10, vec![("v10.sql", &sql_with_down("t10"))]),
    ]);

    let driver = InMemoryDriver::new();
    let migrator = Migrator::from_root(&f.root);
    migrator.up(&driver).await.expect("up() should succeed");

    let order = driver.apply_order();
    assert_eq!(order.len(), 3);
    assert_eq!(order[0].0, 1, "version 1 first");
    assert_eq!(order[1].0, 2, "version 2 second");
    assert_eq!(order[2].0, 10, "version 10 third (numeric, not after 1)");
}

/// status() reports all migrations as pending when nothing is applied yet.
#[tokio::test]
async fn test_in_memory_status_all_pending_when_applied_empty() {
    let f = Fixture::build(&[(
        1,
        vec![
            ("a.sql", &sql_with_down("ta")),
            ("b.sql", &sql_with_down("tb")),
        ],
    )]);

    let driver = InMemoryDriver::new();
    let migrator = Migrator::from_root(&f.root);

    let status: MigrationStatus = migrator
        .status(&driver)
        .await
        .expect("status() should succeed");

    assert_eq!(status.applied.len(), 0, "driver reports nothing applied");
    assert_eq!(status.pending.len(), 2, "both migrations are pending");
    assert!(status.drift_errors.is_empty(), "no drift expected");
}

/// down(1) reverts the LAST migration in manifest order, leaving the first applied.
///
/// This proves the full external-implementability of `MigrationDriver`:
/// `AppliedMigration::new` builds real rows, `applied()` returns them, and
/// `down()` uses manifest-position ordering to revert in reverse.
#[tokio::test]
async fn test_in_memory_rollback_order() {
    // Two migrations in manifest order: first.sql then second.sql.
    let f = Fixture::build(&[(
        1,
        vec![
            ("first.sql", &sql_with_down("t_first")),
            ("second.sql", &sql_with_down("t_second")),
        ],
    )]);

    let driver = InMemoryDriver::new();
    let migrator = Migrator::from_root(&f.root);

    // Apply both.
    migrator.up(&driver).await.expect("up() should succeed");
    assert_eq!(driver.apply_order().len(), 2, "both migrations applied");

    // Revert one step — must revert the LAST in manifest order (second.sql).
    migrator
        .down(&driver, 1)
        .await
        .expect("down(1) should succeed");

    let reverted = driver.revert_order();
    assert_eq!(reverted.len(), 1, "exactly one migration reverted");
    assert_eq!(
        reverted[0].1, "second.sql",
        "last-in-manifest reverted first"
    );

    // Status: 1 applied (first.sql), 1 pending (second.sql).
    let status = migrator
        .status(&driver)
        .await
        .expect("status() should succeed");
    assert_eq!(status.applied.len(), 1, "one migration still applied");
    assert_eq!(
        status.applied[0].file, "first.sql",
        "first.sql remains applied"
    );
    assert_eq!(status.pending.len(), 1, "second.sql is now pending");
    assert_eq!(
        status.pending[0].file, "second.sql",
        "second.sql is pending"
    );
    assert!(status.drift_errors.is_empty(), "no drift errors");
}

/// The trait is object-safe: Migrator accepts &dyn MigrationDriver.
#[tokio::test]
async fn test_in_memory_trait_object_safety() {
    let f = Fixture::build(&[(1, vec![("m.sql", &sql_with_down("t_obj"))])]);

    // Erase to trait object explicitly.
    let driver: Box<dyn MigrationDriver> = Box::new(InMemoryDriver::new());
    let migrator = Migrator::from_root(&f.root);
    // Call through the trait object.
    migrator
        .up(driver.as_ref())
        .await
        .expect("up() via &dyn MigrationDriver should work");
}
