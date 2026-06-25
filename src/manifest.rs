use serde::Deserialize;
use std::collections::HashSet;

use crate::error::{Error, Result};

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub manifest_version: u32,
    pub versions: Vec<ManifestVersion>,
}

#[derive(Debug, Deserialize)]
pub struct ManifestVersion {
    pub version: u32,
    pub description: Option<String>,
    pub migrations: Vec<ManifestEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ManifestEntry {
    pub file: String,
    pub created: Option<String>,
    pub author: Option<String>,
    pub why: Option<String>,
}

/// Validate that `file` is a bare filename: ends in .sql, no path separators, no "..".
fn validate_filename(file: &str) -> Result<()> {
    if file.contains('/') || file.contains('\\') || file.contains("..") || !file.ends_with(".sql") {
        return Err(Error::InvalidFileName(file.to_owned()));
    }
    Ok(())
}

impl Manifest {
    /// Parse and validate a manifest from YAML text.
    ///
    /// # Errors
    ///
    /// Returns `Error::UnsupportedManifestVersion` if `manifest_version != 1`.
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let manifest: Manifest =
            serde_yml::from_str(yaml).map_err(|e| Error::ManifestParse(e.to_string()))?;

        if manifest.manifest_version != 1 {
            return Err(Error::UnsupportedManifestVersion {
                found: manifest.manifest_version,
            });
        }

        let mut seen: HashSet<(u32, &str)> = HashSet::new();
        for mv in &manifest.versions {
            for entry in &mv.migrations {
                validate_filename(&entry.file)?;
                let key = (mv.version, entry.file.as_str());
                if !seen.insert(key) {
                    return Err(Error::DuplicateEntry {
                        version: mv.version,
                        file: entry.file.clone(),
                    });
                }
            }
        }

        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_manifest() {
        let yaml = r#"
manifest_version: 1
versions:
  - version: 1
    description: "Initial"
    migrations:
      - file: "20260101_01_init.sql"
        created: "2026-01-01"
        why: "Bootstrap"
"#;
        let m = Manifest::from_yaml(yaml).unwrap();
        assert_eq!(m.manifest_version, 1);
        assert_eq!(m.versions.len(), 1);
        assert_eq!(m.versions[0].migrations[0].file, "20260101_01_init.sql");
    }

    #[test]
    fn rejects_path_separator() {
        let yaml = r#"
manifest_version: 1
versions:
  - version: 1
    migrations:
      - file: "../evil.sql"
"#;
        assert!(matches!(
            Manifest::from_yaml(yaml),
            Err(Error::InvalidFileName(_))
        ));
    }

    #[test]
    fn rejects_unsupported_manifest_version() {
        let yaml = r#"
manifest_version: 2
versions: []
"#;
        assert!(matches!(
            Manifest::from_yaml(yaml),
            Err(Error::UnsupportedManifestVersion { found: 2 })
        ));
    }

    #[test]
    fn rejects_duplicate_entry() {
        let yaml = r#"
manifest_version: 1
versions:
  - version: 1
    migrations:
      - file: "20260101_01_init.sql"
      - file: "20260101_01_init.sql"
"#;
        assert!(matches!(
            Manifest::from_yaml(yaml),
            Err(Error::DuplicateEntry { .. })
        ));
    }
}
