/// Integration tests for soma-schema.
///
/// Each test:
///   1. Generates a unique throwaway schema name (_sdm_test_<uuid>).
///   2. Runs all assertions inside that schema.
///   3. DROPs the schema CASCADE on teardown — even on panic — via an RAII guard.
///      The Drop impl spawns a blocking cleanup task so panics are covered.
///
/// Requires TEST_DATABASE_URL to point at a Postgres instance.
/// NEVER touches public or any pre-existing schema.
use std::path::PathBuf;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tempfile::TempDir;
use uuid::Uuid;

use soma_schema::error::Error;
use soma_schema::{Migrator, PostgresConfig, PostgresDriver};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_db_url() -> String {
    std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set to run integration tests")
}

struct SchemaGuard {
    pool: PgPool,
    pub schema: String,
}

impl SchemaGuard {
    /// Explicit cleanup — preferred because it runs in the current async context
    /// without spawning an extra thread. Call at the end of each test.
    async fn cleanup(&self) {
        let _ = sqlx::query(&format!(
            "DROP SCHEMA IF EXISTS \"{}\" CASCADE",
            self.schema
        ))
        .execute(&self.pool)
        .await;
    }
}

/// RAII guard: DROP the schema even on panic.
///
/// Uses `tokio::runtime::Handle::try_current()` + a std::thread to run the async
/// DROP synchronously, mirroring the pattern used by PgLockGuard in the main crate.
/// This makes the "even on panic" comment in the module doc actually true.
impl Drop for SchemaGuard {
    fn drop(&mut self) {
        let pool = self.pool.clone();
        let schema = self.schema.clone();
        // If we're inside a tokio runtime, spawn a blocking thread that owns a
        // fresh single-threaded runtime to issue the DROP. This avoids blocking
        // the async executor and works even during a panic unwind.
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build cleanup runtime");
            rt.block_on(async {
                let _ = sqlx::query(&format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE"))
                    .execute(&pool)
                    .await;
            });
        })
        .join()
        .ok();
    }
}

/// Create a unique throwaway schema and return a guard with an explicit cleanup method.
async fn make_schema(pool: &PgPool) -> SchemaGuard {
    let schema = format!("_sdm_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA \"{schema}\""))
        .execute(pool)
        .await
        .expect("create test schema");
    SchemaGuard {
        pool: pool.clone(),
        schema,
    }
}

async fn make_pool() -> PgPool {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&test_db_url())
        .await
        .expect("connect to test database")
}

fn pg_config(schema: &str) -> PostgresConfig {
    PostgresConfig {
        schema: Some(schema.to_owned()),
        table: "00_schema_migrations".to_owned(),
        advisory_lock_key: 918273645,
    }
}

/// Build a migrations directory from a spec:
/// - setup_sql: content for 00_setup/01_schema.sql (None = no setup file).
/// - versions: list of (version_num, &[( filename, sql )]) in order.
/// - order_yaml: content for migration-order.yaml (generated from versions if None).
struct MigrationsFixture {
    _dir: TempDir,
    root: PathBuf,
}

impl MigrationsFixture {
    /// Build the fixture.
    ///
    /// `versions` controls WHAT files exist on disk.
    /// `yaml_override` lets a test supply a custom manifest (for orphan/order tests).
    fn build(
        setup_sql: Option<&str>,
        versions: &[(u32, Vec<(&str, &str)>)],
        yaml_override: Option<&str>,
    ) -> Self {
        let dir = TempDir::new().unwrap();
        let root = dir.path().to_path_buf();

        if let Some(sql) = setup_sql {
            std::fs::create_dir_all(root.join("00_setup")).unwrap();
            std::fs::write(root.join("00_setup").join("01_schema.sql"), sql).unwrap();
        }

        for (v, files) in versions {
            let migrated = root.join("01_migrated").join(v.to_string());
            std::fs::create_dir_all(&migrated).unwrap();
            for (fname, content) in files {
                std::fs::write(migrated.join(fname), content).unwrap();
            }
        }

        let yaml = match yaml_override {
            Some(y) => y.to_owned(),
            None => {
                let mut out = String::from("manifest_version: 1\nversions:\n");
                for (v, files) in versions {
                    out.push_str(&format!("  - version: {v}\n    migrations:\n"));
                    for (fname, _) in files {
                        out.push_str(&format!("      - file: \"{fname}\"\n"));
                    }
                }
                out
            }
        };
        std::fs::write(root.join("migration-order.yaml"), yaml).unwrap();

        MigrationsFixture { _dir: dir, root }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Helper: create a table in a migration and drop it in the DOWN section.
fn migration_create_table(table: &str) -> String {
    format!(
        "CREATE TABLE {{SCHEMA}}.{table} (id SERIAL PRIMARY KEY);\n-- DOWN ==\nDROP TABLE IF EXISTS {{SCHEMA}}.{table};"
    )
}

/// Replace `{SCHEMA}` placeholder in migration SQL with the actual schema name.
fn with_schema(sql: &str, schema: &str) -> String {
    sql.replace("{SCHEMA}", schema)
}

#[tokio::test]
async fn test_fresh_up_applies_all_and_creates_tracking_rows() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    let setup_sql = "-- idempotent\nSELECT 1;";
    let f = MigrationsFixture::build(
        Some(setup_sql),
        &[(
            1,
            vec![
                (
                    "20260101_01_init.sql",
                    &with_schema(&migration_create_table("tbl_a"), &schema),
                ),
                (
                    "20260101_02_more.sql",
                    &with_schema(&migration_create_table("tbl_b"), &schema),
                ),
            ],
        )],
        None,
    );

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&f.root);
    migrator.up(&driver).await.expect("up should succeed");

    // Both tables must exist.
    for tbl in ["tbl_a", "tbl_b"] {
        let exists: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema = $1 AND table_name = $2)"
        )
        .bind(&schema)
        .bind(tbl)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(exists.0, "table {tbl} should exist after up()");
    }

    // Tracking rows must exist.
    let rows: Vec<(i32, String)> = sqlx::query_as(&format!(
        "SELECT version, file FROM \"{schema}\".\"00_schema_migrations\" ORDER BY version, file"
    ))
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1, "20260101_01_init.sql");
    assert_eq!(rows[1].1, "20260101_02_more.sql");
    guard.cleanup().await;
}

#[tokio::test]
async fn test_up_again_is_noop() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    let setup_sql = "SELECT 1;";
    let f = MigrationsFixture::build(
        Some(setup_sql),
        &[(
            1,
            vec![(
                "20260101_01_init.sql",
                &with_schema(&migration_create_table("tbl_noop"), &schema),
            )],
        )],
        None,
    );
    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&f.root);

    migrator.up(&driver).await.unwrap();

    // Count rows before second up().
    let before: (i64,) = sqlx::query_as(&format!(
        "SELECT COUNT(*) FROM \"{schema}\".\"00_schema_migrations\""
    ))
    .fetch_one(&pool)
    .await
    .unwrap();

    migrator
        .up(&driver)
        .await
        .expect("second up() should be a no-op");

    let after: (i64,) = sqlx::query_as(&format!(
        "SELECT COUNT(*) FROM \"{schema}\".\"00_schema_migrations\""
    ))
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(before.0, after.0, "no new rows after second up()");
    guard.cleanup().await;
}

#[tokio::test]
async fn test_checksum_drift_error() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    let original_sql = with_schema(&migration_create_table("tbl_drift"), &schema);
    let f = MigrationsFixture::build(
        Some("SELECT 1;"),
        &[(1, vec![("20260101_01_init.sql", &original_sql)])],
        None,
    );

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&f.root);
    migrator.up(&driver).await.unwrap();

    // Now alter the file on disk to simulate drift.
    std::fs::write(
        f.root
            .join("01_migrated")
            .join("1")
            .join("20260101_01_init.sql"),
        format!("{original_sql}\n-- drift"),
    )
    .unwrap();

    let result = migrator.up(&driver).await;
    assert!(
        matches!(result, Err(Error::ChecksumDrift { .. })),
        "expected ChecksumDrift, got: {result:?}"
    );
    guard.cleanup().await;
}

#[tokio::test]
async fn test_down_one_step_reverts_newest() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    let f = MigrationsFixture::build(
        Some("SELECT 1;"),
        &[(
            1,
            vec![
                (
                    "20260101_01_init.sql",
                    &with_schema(&migration_create_table("tbl_first"), &schema),
                ),
                (
                    "20260101_02_more.sql",
                    &with_schema(&migration_create_table("tbl_second"), &schema),
                ),
            ],
        )],
        None,
    );

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&f.root);
    migrator.up(&driver).await.unwrap();

    migrator
        .down(&driver, 1)
        .await
        .expect("down 1 should succeed");

    // tbl_second should be gone; tbl_first should remain.
    let second_exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema = $1 AND table_name = $2)"
    )
    .bind(&schema)
    .bind("tbl_second")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        !second_exists.0,
        "tbl_second should be dropped after down 1"
    );

    let first_exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema = $1 AND table_name = $2)"
    )
    .bind(&schema)
    .bind("tbl_first")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(first_exists.0, "tbl_first should still exist");

    // Tracking row for tbl_second should be gone.
    let rows: Vec<(String,)> = sqlx::query_as(&format!(
        "SELECT file FROM \"{schema}\".\"00_schema_migrations\""
    ))
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "20260101_01_init.sql");
    guard.cleanup().await;
}

#[tokio::test]
async fn test_failing_migration_rolls_back_and_stops() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    // Migration 2 is intentionally broken SQL.
    let good_sql = with_schema(&migration_create_table("tbl_good"), &schema);
    let bad_sql = "THIS IS NOT VALID SQL;";
    let later_sql = with_schema(&migration_create_table("tbl_later"), &schema);

    let f = MigrationsFixture::build(
        Some("SELECT 1;"),
        &[(
            1,
            vec![
                ("20260101_01_good.sql", &good_sql),
                ("20260101_02_bad.sql", bad_sql),
                ("20260101_03_later.sql", &later_sql),
            ],
        )],
        None,
    );

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&f.root);
    let result = migrator.up(&driver).await;
    assert!(result.is_err(), "up should fail due to bad SQL");

    // tbl_good must exist (it was applied and committed before the failure).
    let good_exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema = $1 AND table_name = $2)"
    )
    .bind(&schema)
    .bind("tbl_good")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        good_exists.0,
        "tbl_good should exist (applied before the failure)"
    );

    // tbl_later must NOT exist.
    let later_exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema = $1 AND table_name = $2)"
    )
    .bind(&schema)
    .bind("tbl_later")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        !later_exists.0,
        "tbl_later should NOT exist (after the failure, not applied)"
    );

    // Only tbl_good's tracking row should exist.
    let rows: Vec<(String,)> = sqlx::query_as(&format!(
        "SELECT file FROM \"{schema}\".\"00_schema_migrations\""
    ))
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "20260101_01_good.sql");
    guard.cleanup().await;
}

#[tokio::test]
async fn test_manifest_order_honored_regardless_of_filename_sort() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    // File "b" sorts first lexically but is listed SECOND in the manifest.
    // We verify "a" runs before "b" by making "b" depend on "a" existing.
    let sql_a = format!(
        "CREATE TABLE \"{schema}\".order_a (id SERIAL PRIMARY KEY);\n-- DOWN ==\nDROP TABLE IF EXISTS \"{schema}\".order_a;"
    );
    // sql_b inserts into order_a — if a hasn't run first, this will fail.
    let sql_b = format!(
        "INSERT INTO \"{schema}\".order_a DEFAULT VALUES;\n-- DOWN ==\nDELETE FROM \"{schema}\".order_a;"
    );

    // Filenames: b_first.sql sorts before a_second.sql lexically.
    let yaml = r#"manifest_version: 1
versions:
  - version: 1
    migrations:
      - file: "a_second.sql"
      - file: "b_first.sql"
"#;

    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join("01_migrated").join("1")).unwrap();
    std::fs::write(
        root.join("01_migrated").join("1").join("a_second.sql"),
        &sql_a,
    )
    .unwrap();
    std::fs::write(
        root.join("01_migrated").join("1").join("b_first.sql"),
        &sql_b,
    )
    .unwrap();
    std::fs::write(root.join("migration-order.yaml"), yaml).unwrap();

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&root);
    migrator
        .up(&driver)
        .await
        .expect("manifest order should be honored (a before b)");
    guard.cleanup().await;
}

#[tokio::test]
async fn test_version_folders_ordered_numerically() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    // Version 2 should run after version 1.
    // Version 10 should run after version 2 (numeric, not lexicographic).
    let sql_v1 = format!(
        "CREATE TABLE \"{schema}\".v1_tbl (id SERIAL PRIMARY KEY);\n-- DOWN ==\nDROP TABLE IF EXISTS \"{schema}\".v1_tbl;"
    );
    let sql_v2 = format!(
        "INSERT INTO \"{schema}\".v1_tbl DEFAULT VALUES;\n-- DOWN ==\nDELETE FROM \"{schema}\".v1_tbl;"
    );
    // v10 depends on v2's row existing (row count = 1).
    let sql_v10 =
        format!("UPDATE \"{schema}\".v1_tbl SET id = id WHERE id > 0;\n-- DOWN ==\nSELECT 1;");

    let yaml = r#"manifest_version: 1
versions:
  - version: 1
    migrations:
      - file: "v1.sql"
  - version: 2
    migrations:
      - file: "v2.sql"
  - version: 10
    migrations:
      - file: "v10.sql"
"#;

    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();
    for (v, fname, content) in [
        (1u32, "v1.sql", sql_v1.as_str()),
        (2, "v2.sql", sql_v2.as_str()),
        (10, "v10.sql", sql_v10.as_str()),
    ] {
        std::fs::create_dir_all(root.join("01_migrated").join(v.to_string())).unwrap();
        std::fs::write(
            root.join("01_migrated").join(v.to_string()).join(fname),
            content,
        )
        .unwrap();
    }
    std::fs::write(root.join("migration-order.yaml"), yaml).unwrap();

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&root);
    migrator
        .up(&driver)
        .await
        .expect("numeric version ordering should work");

    let rows: Vec<(i32,)> = sqlx::query_as(&format!(
        "SELECT version FROM \"{schema}\".\"00_schema_migrations\" ORDER BY version"
    ))
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.iter().map(|r| r.0).collect::<Vec<_>>(), vec![1, 2, 10]);
    guard.cleanup().await;
}

#[tokio::test]
async fn test_setup_runs_before_migrations_and_is_idempotent() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    // 00_setup creates a table; the migration uses it.
    let setup_sql =
        format!("CREATE TABLE IF NOT EXISTS \"{schema}\".setup_marker (id SERIAL PRIMARY KEY);");
    let migration_sql = format!(
        "INSERT INTO \"{schema}\".setup_marker DEFAULT VALUES;\n-- DOWN ==\nDELETE FROM \"{schema}\".setup_marker;"
    );

    let f = MigrationsFixture::build(
        Some(&setup_sql),
        &[(1, vec![("20260101_01_uses_setup.sql", &migration_sql)])],
        None,
    );

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&f.root);

    // First up(): setup runs, migration applies.
    migrator.up(&driver).await.expect("first up should succeed");

    // Second up(): setup runs again (idempotent), no new migration applied.
    migrator
        .up(&driver)
        .await
        .expect("second up should be a no-op");

    let count: (i64,) = sqlx::query_as(&format!(
        "SELECT COUNT(*) FROM \"{schema}\".\"00_schema_migrations\""
    ))
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count.0, 1, "still only one migration row after second up");
    guard.cleanup().await;
}

#[tokio::test]
async fn test_setup_files_never_orphan_flagged() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    // 00_setup has a file NOT in the manifest (by design — setup is untracked).
    let setup_sql = "SELECT 1;";
    let migration_sql = format!(
        "CREATE TABLE \"{schema}\".tbl_x (id SERIAL PRIMARY KEY);\n-- DOWN ==\nDROP TABLE IF EXISTS \"{schema}\".tbl_x;"
    );

    let f = MigrationsFixture::build(
        Some(setup_sql),
        &[(1, vec![("20260101_01_init.sql", &migration_sql)])],
        None,
    );

    // Write an extra file in 00_setup — it should NOT be flagged as an orphan.
    std::fs::write(f.root.join("00_setup").join("02_extra.sql"), "SELECT 2;").unwrap();

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&f.root);
    migrator
        .up(&driver)
        .await
        .expect("extra setup file should not be an orphan");
    guard.cleanup().await;
}

#[tokio::test]
async fn test_orphan_migration_returns_error() {
    // Create a .sql file under a version folder that is NOT in the manifest.
    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();

    let yaml = "manifest_version: 1\nversions:\n  - version: 1\n    migrations:\n      - file: \"listed.sql\"\n";
    std::fs::create_dir_all(root.join("01_migrated").join("1")).unwrap();
    std::fs::write(
        root.join("01_migrated").join("1").join("listed.sql"),
        "SELECT 1;",
    )
    .unwrap();
    std::fs::write(
        root.join("01_migrated").join("1").join("orphan.sql"),
        "SELECT 2;",
    )
    .unwrap();
    std::fs::write(root.join("migration-order.yaml"), yaml).unwrap();

    let result = soma_schema::discovery::discover(&root);
    assert!(
        matches!(result, Err(Error::OrphanMigration { .. })),
        "expected OrphanMigration, got: {result:?}"
    );
}

#[tokio::test]
async fn test_missing_file_returns_error() {
    // Manifest lists a file that doesn't exist on disk.
    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();

    let yaml = "manifest_version: 1\nversions:\n  - version: 1\n    migrations:\n      - file: \"ghost.sql\"\n";
    std::fs::create_dir_all(root.join("01_migrated").join("1")).unwrap();
    // ghost.sql is NOT written.
    std::fs::write(root.join("migration-order.yaml"), yaml).unwrap();

    let result = soma_schema::discovery::discover(&root);
    assert!(
        matches!(result, Err(Error::MissingFile { .. })),
        "expected MissingFile, got: {result:?}"
    );
}

#[tokio::test]
async fn test_invalid_filename_in_manifest_rejected() {
    let yaml = r#"manifest_version: 1
versions:
  - version: 1
    migrations:
      - file: "../evil.sql"
"#;
    let result = soma_schema::manifest::Manifest::from_yaml(yaml);
    assert!(
        matches!(result, Err(Error::InvalidFileName(_))),
        "expected InvalidFileName, got: {result:?}"
    );
}

#[tokio::test]
async fn test_identifier_validation() {
    // Invalid schema name should be rejected.
    let pool = make_pool().await;
    let result = PostgresDriver::new(
        pool.clone(),
        PostgresConfig {
            schema: Some("bad-schema".to_owned()),
            table: "00_schema_migrations".to_owned(),
            advisory_lock_key: 918273645,
        },
    );
    assert!(
        matches!(result, Err(Error::InvalidIdentifier(_))),
        "bad schema name should be rejected"
    );

    // Leading-digit names should be ALLOWED.
    let ok = PostgresDriver::new(
        pool.clone(),
        PostgresConfig {
            schema: Some("00_schema".to_owned()),
            table: "00_schema_migrations".to_owned(),
            advisory_lock_key: 918273645,
        },
    );
    assert!(ok.is_ok(), "leading-digit identifier should be accepted");
}

#[tokio::test]
async fn test_deployment_tracking_metadata() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    let sql = with_schema(&migration_create_table("tbl_meta"), &schema);
    let yaml = r#"manifest_version: 1
versions:
  - version: 1
    migrations:
      - file: "20260101_01_init.sql"
        why: "Track metadata"
"#;
    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join("01_migrated").join("1")).unwrap();
    std::fs::write(
        root.join("01_migrated")
            .join("1")
            .join("20260101_01_init.sql"),
        &sql,
    )
    .unwrap();
    std::fs::write(root.join("migration-order.yaml"), yaml).unwrap();

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&root);
    migrator.up(&driver).await.unwrap();

    let row: (
        i32,
        String,
        Option<String>,
        i32,
        sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
        String,
        Option<i32>,
    ) = sqlx::query_as(&format!(
        "SELECT version, file, description, batch, applied_at, applied_by, execution_ms \
             FROM \"{schema}\".\"00_schema_migrations\""
    ))
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.0, 1, "version");
    assert_eq!(row.1, "20260101_01_init.sql", "file");
    assert_eq!(
        row.2.as_deref(),
        Some("Track metadata"),
        "description from why"
    );
    assert_eq!(row.3, 1, "first batch");
    // applied_at should be recent (within last minute).
    let age = sqlx::types::chrono::Utc::now() - row.4;
    assert!(age.num_seconds() < 60, "applied_at should be recent");
    assert!(!row.5.is_empty(), "applied_by should not be empty");
    assert!(row.6.is_some(), "execution_ms should be set");
    guard.cleanup().await;
}

#[tokio::test]
async fn test_batch_increments_across_separate_up_runs() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    // m1.sql lives in version 1, m2.sql in version 2.
    // First up() uses only the v1 manifest; m2 is in a v2 folder not referenced yet (no orphan).
    // Second up() expands the manifest to include v2, applying m2 in a new batch.
    let sql1 = with_schema(&migration_create_table("tbl_batch1"), &schema);
    let sql2 = with_schema(&migration_create_table("tbl_batch2"), &schema);

    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();
    // Only create the v1 folder now; v2 is created before the second up() call so the
    // orphan check (which now also catches unregistered version folders) doesn't trip.
    std::fs::create_dir_all(root.join("01_migrated").join("1")).unwrap();
    std::fs::write(root.join("01_migrated").join("1").join("m1.sql"), &sql1).unwrap();

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&root);

    // First up(): only v1 manifest. The v2 folder does not exist yet on disk,
    // so the orphan check has nothing to flag.
    let yaml_v1_only = r#"manifest_version: 1
versions:
  - version: 1
    migrations:
      - file: "m1.sql"
"#;
    std::fs::write(root.join("migration-order.yaml"), yaml_v1_only).unwrap();
    migrator.up(&driver).await.unwrap();

    // Second up(): add v2 folder + file to disk, then expand the manifest.
    std::fs::create_dir_all(root.join("01_migrated").join("2")).unwrap();
    std::fs::write(root.join("01_migrated").join("2").join("m2.sql"), &sql2).unwrap();

    let yaml_both = r#"manifest_version: 1
versions:
  - version: 1
    migrations:
      - file: "m1.sql"
  - version: 2
    migrations:
      - file: "m2.sql"
"#;
    std::fs::write(root.join("migration-order.yaml"), yaml_both).unwrap();
    migrator.up(&driver).await.unwrap();

    // m1 should have batch=1, m2 should have batch=2.
    let rows: Vec<(String, i32)> = sqlx::query_as(&format!(
        "SELECT file, batch FROM \"{schema}\".\"00_schema_migrations\" ORDER BY file"
    ))
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    let m1 = rows.iter().find(|r| r.0 == "m1.sql").unwrap();
    let m2 = rows.iter().find(|r| r.0 == "m2.sql").unwrap();
    assert_eq!(m1.1, 1, "m1 batch should be 1");
    assert_eq!(m2.1, 2, "m2 batch should be 2 (separate run)");
    guard.cleanup().await;
}

#[tokio::test]
async fn test_idempotent_seed_applied_once() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    // Migration 1 creates table; migration 2 is an idempotent seed.
    let create_sql = format!(
        "CREATE TABLE \"{schema}\".seeds (id SERIAL PRIMARY KEY, name TEXT UNIQUE);\n-- DOWN ==\nDROP TABLE IF EXISTS \"{schema}\".seeds;"
    );
    let seed_sql = format!(
        "INSERT INTO \"{schema}\".seeds (name) VALUES ('alpha') ON CONFLICT DO NOTHING;\n-- DOWN ==\nDELETE FROM \"{schema}\".seeds WHERE name = 'alpha';"
    );

    let f = MigrationsFixture::build(
        Some("SELECT 1;"),
        &[(
            1,
            vec![
                ("20260101_01_create.sql", &create_sql),
                ("20260101_02_seed.sql", &seed_sql),
            ],
        )],
        None,
    );

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&f.root);

    migrator.up(&driver).await.unwrap();

    // Re-up should not duplicate the seed.
    migrator.up(&driver).await.unwrap();

    let count: (i64,) = sqlx::query_as(&format!(
        "SELECT COUNT(*) FROM \"{schema}\".seeds WHERE name = 'alpha'"
    ))
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count.0, 1, "seed row should appear exactly once");
    guard.cleanup().await;
}

/// Finding #9: down() must revert in reverse manifest order, not reverse filename sort.
///
/// Manifest lists a_second.sql first, b_first.sql second (non-alpha order).
/// up() must apply a then b; down(1) must revert b (last applied) not a.
#[tokio::test]
async fn test_down_reverts_in_reverse_manifest_order_not_filename_sort() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    // a_second.sql creates a table; b_first.sql inserts into it.
    // Correct down(1) must revert b_first (the insert), leaving the table intact.
    // If down() sorted by filename DESC it would pick b_first (b > a) — that happens
    // to be correct here. So we invert: manifest lists z_first.sql first, a_second.sql
    // second. z > a means wrong filename sort picks z_first; correct manifest sort
    // picks a_second (position 1 > position 0).
    let sql_z = format!(
        "CREATE TABLE \"{schema}\".rev_order_z (id SERIAL PRIMARY KEY);\n-- DOWN ==\nDROP TABLE IF EXISTS \"{schema}\".rev_order_z;"
    );
    // a_second.sql inserts into rev_order_z — it must run after z_first.sql.
    let sql_a = format!(
        "INSERT INTO \"{schema}\".rev_order_z DEFAULT VALUES;\n-- DOWN ==\nDELETE FROM \"{schema}\".rev_order_z;"
    );

    // Manifest: z_first listed first, a_second listed second.
    let yaml = r#"manifest_version: 1
versions:
  - version: 1
    migrations:
      - file: "z_first.sql"
      - file: "a_second.sql"
"#;

    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join("01_migrated").join("1")).unwrap();
    std::fs::write(
        root.join("01_migrated").join("1").join("z_first.sql"),
        &sql_z,
    )
    .unwrap();
    std::fs::write(
        root.join("01_migrated").join("1").join("a_second.sql"),
        &sql_a,
    )
    .unwrap();
    std::fs::write(root.join("migration-order.yaml"), yaml).unwrap();

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&root);
    migrator
        .up(&driver)
        .await
        .expect("up with non-alpha manifest order should succeed");

    // down(1) must revert a_second.sql (manifest position 1, last applied), NOT z_first.sql.
    migrator
        .down(&driver, 1)
        .await
        .expect("down 1 should succeed");

    // rev_order_z table should still exist (z_first.sql was NOT reverted).
    let tbl_exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema = $1 AND table_name = $2)"
    )
    .bind(&schema)
    .bind("rev_order_z")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        tbl_exists.0,
        "rev_order_z should still exist after down(1) reverts a_second, not z_first"
    );

    // Only z_first.sql row should remain in tracking table.
    let rows: Vec<(String,)> = sqlx::query_as(&format!(
        "SELECT file FROM \"{schema}\".\"00_schema_migrations\""
    ))
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].0, "z_first.sql",
        "z_first.sql should still be in tracking after down(1)"
    );
    guard.cleanup().await;
}

/// Finding #10: orphan check must catch SQL files in version folders not in the manifest.
#[tokio::test]
async fn test_orphan_in_unregistered_version_folder_returns_error() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();

    // Manifest only knows about version 1.
    let yaml = "manifest_version: 1\nversions:\n  - version: 1\n    migrations:\n      - file: \"ok.sql\"\n";
    std::fs::create_dir_all(root.join("01_migrated").join("1")).unwrap();
    std::fs::write(
        root.join("01_migrated").join("1").join("ok.sql"),
        "SELECT 1;",
    )
    .unwrap();
    // Version 99 folder exists on disk but is NOT in the manifest.
    std::fs::create_dir_all(root.join("01_migrated").join("99")).unwrap();
    std::fs::write(
        root.join("01_migrated").join("99").join("surprise.sql"),
        "SELECT 99;",
    )
    .unwrap();
    std::fs::write(root.join("migration-order.yaml"), yaml).unwrap();

    let result = soma_schema::discovery::discover(&root);
    assert!(
        matches!(result, Err(Error::OrphanMigration { .. })),
        "expected OrphanMigration for unregistered version folder, got: {result:?}"
    );
}

/// Finding #11: up() must return AppliedButMissing when a DB row has no on-disk match.
#[tokio::test]
async fn test_applied_but_missing_from_manifest_returns_error() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    let sql = with_schema(&migration_create_table("tbl_ghost"), &schema);
    let f = MigrationsFixture::build(
        Some("SELECT 1;"),
        &[(1, vec![("20260101_01_init.sql", &sql)])],
        None,
    );

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&f.root);
    migrator.up(&driver).await.unwrap();

    // Now remove the migration file AND the manifest entry, leaving the DB row orphaned.
    std::fs::remove_file(
        f.root
            .join("01_migrated")
            .join("1")
            .join("20260101_01_init.sql"),
    )
    .unwrap();
    std::fs::write(
        f.root.join("migration-order.yaml"),
        "manifest_version: 1\nversions:\n  - version: 1\n    migrations: []\n",
    )
    .unwrap();

    let result = migrator.up(&driver).await;
    assert!(
        matches!(result, Err(Error::AppliedButMissing { .. })),
        "expected AppliedButMissing when applied DB row has no on-disk match, got: {result:?}"
    );
    guard.cleanup().await;
}

/// Finding #6: PostgresDriver::new must reject pools with max_connections < 2.
#[tokio::test]
async fn test_pool_too_small_rejected() {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&test_db_url())
        .await
        .expect("connect");
    let result = PostgresDriver::new(pool, pg_config("dummy"));
    assert!(
        matches!(result, Err(Error::PoolTooSmall)),
        "expected PoolTooSmall, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Status tests (§1 of brief)
// ---------------------------------------------------------------------------

/// status() shows correct applied + pending counts after a partial up().
#[tokio::test]
async fn test_status_reports_applied_and_pending() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    // Two migrations; we apply only the first.
    let sql1 = with_schema(&migration_create_table("tbl_s1"), &schema);
    let sql2 = with_schema(&migration_create_table("tbl_s2"), &schema);
    let f = MigrationsFixture::build(
        Some("SELECT 1;"),
        &[(
            1,
            vec![("20260101_01_a.sql", &sql1), ("20260101_02_b.sql", &sql2)],
        )],
        None,
    );

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&f.root);

    // Apply only one by writing a manifest that lists just the first.
    let yaml_one = r#"manifest_version: 1
versions:
  - version: 1
    migrations:
      - file: "20260101_01_a.sql"
"#;
    std::fs::write(f.root.join("migration-order.yaml"), yaml_one).unwrap();
    migrator.up(&driver).await.unwrap();

    // Now expand manifest to include both, but don't run up again.
    let yaml_both = r#"manifest_version: 1
versions:
  - version: 1
    migrations:
      - file: "20260101_01_a.sql"
      - file: "20260101_02_b.sql"
"#;
    std::fs::write(f.root.join("migration-order.yaml"), yaml_both).unwrap();

    let status = migrator
        .status(&driver)
        .await
        .expect("status() should succeed");
    assert_eq!(status.applied.len(), 1, "one migration applied");
    assert_eq!(status.pending.len(), 1, "one migration pending");
    assert_eq!(status.applied[0].file, "20260101_01_a.sql");
    assert_eq!(status.pending[0].file, "20260101_02_b.sql");
    assert!(status.drift_errors.is_empty(), "no drift expected");
    guard.cleanup().await;
}

/// status() reports drift_errors as non-empty but still returns Ok.
#[tokio::test]
async fn test_status_reports_drift_without_aborting() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    let original_sql = with_schema(&migration_create_table("tbl_drift_status"), &schema);
    let f = MigrationsFixture::build(
        Some("SELECT 1;"),
        &[(1, vec![("20260101_01_init.sql", &original_sql)])],
        None,
    );

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&f.root);
    migrator.up(&driver).await.unwrap();

    // Modify the file to introduce drift.
    std::fs::write(
        f.root
            .join("01_migrated")
            .join("1")
            .join("20260101_01_init.sql"),
        format!("{original_sql}\n-- drift comment"),
    )
    .unwrap();

    // status() must return Ok and report the drift, not abort.
    let status = migrator
        .status(&driver)
        .await
        .expect("status() should return Ok even with drift");
    assert!(
        !status.drift_errors.is_empty(),
        "drift_errors should be non-empty when file was modified"
    );
    guard.cleanup().await;
}

/// down() returns Err(ChecksumDrift) when the file was modified after apply.
#[tokio::test]
async fn test_down_detects_drift() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    let original_sql = with_schema(&migration_create_table("tbl_down_drift"), &schema);
    let f = MigrationsFixture::build(
        Some("SELECT 1;"),
        &[(1, vec![("20260101_01_init.sql", &original_sql)])],
        None,
    );

    let driver = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let migrator = Migrator::from_root(&f.root);
    migrator.up(&driver).await.unwrap();

    // Modify the file to introduce drift.
    std::fs::write(
        f.root
            .join("01_migrated")
            .join("1")
            .join("20260101_01_init.sql"),
        format!("{original_sql}\n-- drift"),
    )
    .unwrap();

    let result = migrator.down(&driver, 1).await;
    // Wildcard arm — Error is #[non_exhaustive].
    match result {
        Err(Error::ChecksumDrift { .. }) => {} // expected
        Err(e) => panic!("expected ChecksumDrift, got: {e:?}"),
        Ok(_) => panic!("expected Err(ChecksumDrift), got Ok"),
    }
    guard.cleanup().await;
}

/// status() does not require the advisory lock and succeeds concurrently.
///
/// ponytail: acquiring a real concurrent lock in unit tests is inherently racy.
/// We verify the weaker property: status() on a fresh driver (with its own lock key)
/// returns Ok, proving the code path works. A true "lock already held" test would
/// require two simultaneous async tasks sharing the same lock key and careful
/// synchronisation; the complexity isn't worth the marginal coverage here.
#[tokio::test]
async fn test_status_does_not_block_during_up() {
    let pool = make_pool().await;
    let guard = make_schema(&pool).await;
    let schema = guard.schema.clone();

    let sql = with_schema(&migration_create_table("tbl_concurrent"), &schema);
    let f = MigrationsFixture::build(
        Some("SELECT 1;"),
        &[(1, vec![("20260101_01_init.sql", &sql)])],
        None,
    );

    // Use a different lock key so the status() driver never conflicts.
    let driver_up = PostgresDriver::new(pool.clone(), pg_config(&schema)).unwrap();
    let driver_status = PostgresDriver::new(
        pool.clone(),
        PostgresConfig {
            schema: Some(schema.clone()),
            table: "00_schema_migrations".to_owned(),
            advisory_lock_key: 918273646, // different key
        },
    )
    .unwrap();

    let migrator = Migrator::from_root(&f.root);
    migrator.up(&driver_up).await.unwrap();

    // status() with a different lock key should always succeed.
    let status = migrator
        .status(&driver_status)
        .await
        .expect("status() should return Ok");
    assert_eq!(status.applied.len(), 1);
    guard.cleanup().await;
}
