use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::manifest::Manifest;
use crate::migration::{Migration, SetupFile, read_and_checksum};

/// Load migration metadata (checksums only — no SQL content) and setup file paths.
///
/// The migrations root must contain:
/// - `migration-order.yaml`
/// - `00_setup/*.sql`          (untracked bootstrap, never orphan-flagged)
/// - `01_migrated/<v>/<file>`  (immutable deployed migrations)
/// - `02_inprogress/<v>/<file>` (WIP migrations, same version structure)
///
/// SQL content is NOT loaded here — the migrator reads it lazily at apply/revert time.
pub fn discover(root: &Path) -> Result<(Vec<Migration>, Vec<SetupFile>)> {
    let manifest_path = root.join("migration-order.yaml");
    if !manifest_path.exists() {
        return Err(Error::ManifestMissing);
    }
    let yaml = std::fs::read_to_string(&manifest_path)?;
    let manifest = Manifest::from_yaml(&yaml)?;

    // Collect all on-disk sql files under 01_migrated and 02_inprogress (per version).
    // Track which ones the manifest references so we can detect orphans.
    let mut manifest_set: HashSet<(u32, String)> = HashSet::new();

    // Sort versions numerically in place — no clone needed.
    let mut versions = manifest.versions;
    versions.sort_by_key(|v| v.version);

    let mut migrations: Vec<Migration> = Vec::new();

    for mv in &versions {
        let v = mv.version;
        for entry in &mv.migrations {
            manifest_set.insert((v, entry.file.clone()));

            let migrated_path = root
                .join("01_migrated")
                .join(v.to_string())
                .join(&entry.file);
            let inprogress_path = root
                .join("02_inprogress")
                .join(v.to_string())
                .join(&entry.file);

            let file_path = if migrated_path.exists() {
                migrated_path
            } else if inprogress_path.exists() {
                inprogress_path
            } else {
                return Err(Error::MissingFile {
                    version: v,
                    file: entry.file.clone(),
                });
            };

            // Single atomic read: checksum and raw SQL come from the same bytes.
            let (checksum, raw) = read_and_checksum(&file_path)?;
            migrations.push(Migration::from_metadata(
                v,
                &entry.file,
                checksum,
                raw,
                entry.created.as_deref(),
                entry.author.as_deref(),
                entry.why.as_deref(),
            ));
        }

        // Orphan check: walk all .sql files on disk for this version.
        for dir in [
            root.join("01_migrated").join(v.to_string()),
            root.join("02_inprogress").join(v.to_string()),
        ] {
            if !dir.exists() {
                continue;
            }
            let disk_files = collect_sql_files(&dir)?;
            for disk_file in disk_files {
                let fname = disk_file
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("")
                    .to_owned();
                if !manifest_set.contains(&(v, fname.clone())) {
                    return Err(Error::OrphanMigration {
                        path: disk_file.display().to_string(),
                    });
                }
            }
        }
    }

    // Orphan check for version folders that exist on disk but are NOT in the manifest.
    // Without this, a developer could create 01_migrated/99/ with SQL files and the
    // manifest-keyed loop above would never descend into it.
    let manifest_versions: HashSet<u32> = versions.iter().map(|v| v.version).collect();
    for base in [root.join("01_migrated"), root.join("02_inprogress")] {
        if !base.exists() {
            continue;
        }
        for entry in std::fs::read_dir(&base)? {
            let entry = entry?;
            let dir_path = entry.path();
            if !dir_path.is_dir() {
                continue;
            }
            // Only numeric subdirectory names are version folders.
            let v: u32 = match dir_path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.parse().ok())
            {
                Some(v) => v,
                None => continue,
            };
            if manifest_versions.contains(&v) {
                // Already handled by the per-manifest-version loop above.
                continue;
            }
            // Version folder exists on disk but is not in the manifest — all SQL
            // files inside it are orphans.
            let sql_files = collect_sql_files(&dir_path)?;
            if let Some(f) = sql_files.into_iter().next() {
                return Err(Error::OrphanMigration {
                    path: f.display().to_string(),
                });
            }
        }
    }

    // Collect 00_setup file paths in filename order (untracked, never orphan-flagged).
    // Content is read lazily at run time.
    let setup_dir = root.join("00_setup");
    let mut setup_files: Vec<SetupFile> = Vec::new();
    if setup_dir.exists() {
        let mut paths = collect_sql_files(&setup_dir)?;
        paths.sort();
        for path in paths {
            let name = path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("")
                .to_owned();
            setup_files.push(SetupFile { name, path });
        }
    }

    Ok((migrations, setup_files))
}

fn collect_sql_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("sql") {
            out.push(path);
        }
    }
    Ok(out)
}
