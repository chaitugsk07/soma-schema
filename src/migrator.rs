use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::discovery::discover;
use crate::driver::{AppliedMigration, MigrationDriver};
use crate::error::{Error, Result};

/// A pending migration (in manifest order, not yet applied).
#[derive(Debug)]
pub struct PendingMigration {
    pub version: u32,
    pub file: String,
    pub why: Option<String>,
    pub created: Option<String>,
    pub author: Option<String>,
    /// Version description from the manifest (for status display).
    pub version_description: Option<String>,
}

/// The status of all migrations.
#[derive(Debug)]
pub struct MigrationStatus {
    pub applied: Vec<AppliedMigration>,
    pub pending: Vec<PendingMigration>,
}

pub struct Migrator {
    root: PathBuf,
}

impl Migrator {
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Apply all pending migrations in manifest order.
    ///
    /// Steps:
    /// 1. Acquire advisory lock.
    /// 2. Run all 00_setup files (untracked, idempotent).
    /// 3. Ensure tracking table exists.
    /// 4. Load applied migrations; verify checksums.
    /// 5. Apply each pending migration in manifest order; stop on first error.
    ///    SQL content comes from the same file read that computed the checksum (no TOCTOU).
    pub async fn up(&self, driver: &dyn MigrationDriver) -> Result<()> {
        let _lock = driver.acquire_lock().await?;

        let (migrations, setup_files) = discover(&self.root)?;

        // Run 00_setup files (idempotent bootstrap) — content read lazily here.
        for sf in &setup_files {
            let sql = sf.read_sql()?;
            driver.run_setup_sql(&sf.name, &sql).await?;
        }

        driver.ensure_tracking_table().await?;

        let applied = driver.applied().await?;

        // Verify checksums of already-applied migrations.
        // If a migration is in the DB but no longer on disk / in the manifest, that is
        // an explicit integrity violation — surface it rather than silently skipping.
        for am in &applied {
            match migrations
                .iter()
                .find(|m| m.version == am.version && m.file == am.file)
            {
                None => {
                    return Err(Error::AppliedButMissing {
                        version: am.version,
                        file: am.file.clone(),
                    });
                }
                Some(m) if m.checksum != am.checksum => {
                    return Err(Error::ChecksumDrift {
                        version: am.version,
                        file: am.file.clone(),
                    });
                }
                Some(_) => {}
            }
        }

        // Compute next batch number.
        let batch = applied.iter().map(|a| a.batch).max().unwrap_or(0) + 1;

        // Determine which migrations are pending (in manifest order).
        let applied_keys: std::collections::HashSet<(u32, &str)> = applied
            .iter()
            .map(|a| (a.version, a.file.as_str()))
            .collect();

        for migration in &migrations {
            if !applied_keys.contains(&(migration.version, migration.file.as_str())) {
                // SQL content comes from the stored raw field (same read as checksum).
                let (up_sql, _) = migration.read_up();
                driver.apply(migration, &up_sql, batch).await?;
            }
        }

        Ok(())
    }

    /// Revert the last `steps` applied migrations in reverse manifest order.
    ///
    /// Revert order is the exact reverse of the manifest-defined apply order:
    /// the migration with the highest manifest position is reverted first.
    /// This guarantees FK safety regardless of filename naming conventions.
    pub async fn down(&self, driver: &dyn MigrationDriver, steps: usize) -> Result<()> {
        if steps == 0 {
            return Ok(());
        }

        let _lock = driver.acquire_lock().await?;

        let (migrations, _setup_files) = discover(&self.root)?;

        // Ensure the tracking table exists so applied() doesn't crash on a fresh DB.
        driver.ensure_tracking_table().await?;

        let applied = driver.applied().await?;

        // Integrity check ALL applied migrations before reverting any — same guard as up().
        // Without this, applied migrations outside the take(steps) window would be silently
        // skipped even if their files are gone from the manifest.
        for am in &applied {
            if migrations
                .iter()
                .find(|m| m.version == am.version && m.file == am.file)
                .is_none()
            {
                return Err(Error::AppliedButMissing {
                    version: am.version,
                    file: am.file.clone(),
                });
            }
        }

        // Build a position map from manifest order so revert order is deterministic
        // and independent of filename lexicographic order.
        let position: HashMap<(u32, &str), usize> = migrations
            .iter()
            .enumerate()
            .map(|(i, m)| ((m.version, m.file.as_str()), i))
            .collect();

        let mut applied = applied;
        // Sort descending by manifest position — the last-applied migration reverts first.
        applied.sort_by(|a, b| {
            let pos_a = position
                .get(&(a.version, a.file.as_str()))
                .copied()
                .unwrap_or(0);
            let pos_b = position
                .get(&(b.version, b.file.as_str()))
                .copied()
                .unwrap_or(0);
            pos_b.cmp(&pos_a)
        });

        for am in applied.iter().take(steps) {
            let migration = migrations
                .iter()
                .find(|m| m.version == am.version && m.file == am.file)
                .ok_or_else(|| Error::MissingFile {
                    version: am.version,
                    file: am.file.clone(),
                })?;

            // Guard against modified DOWN SQL after the migration was applied.
            if migration.checksum != am.checksum {
                return Err(Error::ChecksumDrift {
                    version: am.version,
                    file: am.file.clone(),
                });
            }

            // SQL content comes from the stored raw field (same read as checksum).
            let down_sql = migration.read_down().ok_or_else(|| Error::MissingDown {
                version: am.version,
                file: am.file.clone(),
            })?;

            driver.revert(am, &down_sql).await?;
        }

        Ok(())
    }

    /// Return the current migration status: applied + pending.
    pub async fn status(&self, driver: &dyn MigrationDriver) -> Result<MigrationStatus> {
        let _lock = driver.acquire_lock().await?;

        // We need version descriptions from the manifest for the status display.
        let manifest_path = self.root.join("migration-order.yaml");
        let yaml = std::fs::read_to_string(&manifest_path)?;
        let manifest = crate::manifest::Manifest::from_yaml(&yaml)?;
        // Build version -> description map.
        let version_desc: HashMap<u32, Option<String>> = manifest
            .versions
            .into_iter()
            .map(|mv| (mv.version, mv.description))
            .collect();

        let (migrations, _setup_files) = discover(&self.root)?;

        driver.ensure_tracking_table().await?;

        let applied = driver.applied().await?;

        let applied_keys: std::collections::HashSet<(u32, &str)> = applied
            .iter()
            .map(|a| (a.version, a.file.as_str()))
            .collect();

        let pending: Vec<PendingMigration> = migrations
            .iter()
            .filter(|m| !applied_keys.contains(&(m.version, m.file.as_str())))
            .map(|m| PendingMigration {
                version: m.version,
                file: m.file.clone(),
                why: m.why.clone(),
                created: m.created.clone(),
                author: m.author.clone(),
                version_description: version_desc.get(&m.version).and_then(|d| d.clone()),
            })
            .collect();

        Ok(MigrationStatus { applied, pending })
    }

    /// Scaffold a new migrations root directory.
    ///
    /// Creates:
    /// - `migration-order.yaml` (with header comments)
    /// - `00_setup/01_schema.sql` (stub)
    /// - `01_migrated/1/` (empty, ready for the first migration)
    pub fn scaffold(root: &Path) -> Result<()> {
        std::fs::create_dir_all(root.join("00_setup"))?;
        std::fs::create_dir_all(root.join("01_migrated").join("1"))?;
        std::fs::create_dir_all(root.join("02_inprogress"))?;

        let manifest_path = root.join("migration-order.yaml");
        if !manifest_path.exists() {
            std::fs::write(&manifest_path, MANIFEST_TEMPLATE)?;
        }

        let setup_path = root.join("00_setup").join("01_schema.sql");
        if !setup_path.exists() {
            std::fs::write(&setup_path, SETUP_TEMPLATE)?;
        }

        Ok(())
    }
}

const MANIFEST_TEMPLATE: &str = r#"# migration-order.yaml
# Canonical, ordered record of every migration. The runner executes migrations
# in this exact order: versions ascending, then each migration top-to-bottom.
#
# RULES:
#   - Every .sql file under a version folder MUST be listed here.
#   - Every entry MUST resolve to a real file (01_migrated/<v>/<file> or 02_inprogress/<v>/<file>).
#   - 'file' must be a bare filename (no path separators, no '..', must end in .sql).
#   - (00_setup files are NOT listed here — they are the untracked idempotent bootstrap.)
#
# Version folders are named by a positive integer (1, 2, 3, ...).
# Stay in version 1 until you deliberately move to version 2.
# Sort versions NUMERICALLY (1, 2, ..., 10 — never lexically).
manifest_version: 1
versions:
  - version: 1
    description: "Initial schema"
    migrations: []
    # Add your migrations here, e.g.:
    # - file: "20260101_01_init.sql"
    #   created: "2026-01-01"
    #   author: "you"
    #   why: "Create core tables"
"#;

const SETUP_TEMPLATE: &str = r#"-- 00_setup/01_schema.sql
-- Idempotent project bootstrap. Runs UNCONDITIONALLY before every up(), untracked.
-- Every statement MUST be idempotent: use IF NOT EXISTS, CREATE OR REPLACE, etc.

-- Replace 'myapp' with your application schema name.
CREATE SCHEMA IF NOT EXISTS myapp;

-- Example: enable pgcrypto for gen_random_uuid().
-- CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Example: a shared updated_at trigger function (CREATE OR REPLACE is idempotent).
-- CREATE OR REPLACE FUNCTION fn_update_timestamp()
-- RETURNS TRIGGER LANGUAGE plpgsql AS $$
-- BEGIN
--   NEW.updated_at = now();
--   RETURN NEW;
-- END;
-- $$;
"#;
