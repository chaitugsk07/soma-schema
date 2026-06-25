use std::path::PathBuf;

use sha2::{Digest, Sha256};

use crate::error::Result;

/// The separator line (trimmed) that splits UP from DOWN in a migration file.
const DOWN_SEPARATOR: &str = "-- DOWN ==";

/// Chunk size for streaming SHA-256 computation.
const CHECKSUM_CHUNK: usize = 8192;

/// Read a migration file, compute its SHA-256 checksum, and return both together.
///
/// Returning (checksum, raw_content) atomically from a single read prevents TOCTOU:
/// the checksum and the SQL content are always from the same bytes.
pub(crate) fn read_and_checksum(path: &std::path::Path) -> Result<(String, String)> {
    let raw = std::fs::read_to_string(path)?;
    let mut hasher = Sha256::new();
    for chunk in raw.as_bytes().chunks(CHECKSUM_CHUNK) {
        hasher.update(chunk);
    }
    let hex: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    Ok((hex, raw))
}

/// Split raw SQL text into (up_sql, down_sql).
/// Splits on the first line that is exactly `-- DOWN ==` (trimmed).
fn split_up_down(raw: &str) -> (String, Option<String>) {
    let mut up_lines: Vec<&str> = Vec::new();
    let mut down_lines: Vec<&str> = Vec::new();
    let mut in_down = false;
    for line in raw.lines() {
        if !in_down && line.trim() == DOWN_SEPARATOR {
            in_down = true;
            continue;
        }
        if in_down {
            down_lines.push(line);
        } else {
            up_lines.push(line);
        }
    }
    let up_sql = up_lines.join("\n");
    let down_sql = if in_down { Some(down_lines.join("\n")) } else { None };
    (up_sql, down_sql)
}

/// Lightweight migration metadata — SQL content is stored from the initial read.
/// The single file read that computed the checksum also supplies the raw SQL,
/// eliminating any TOCTOU window between checksum computation and execution.
#[derive(Debug)]
pub struct Migration {
    pub version: u32,
    pub file: String,
    /// Human-readable name derived from the filename (strip .sql suffix).
    pub name: String,
    /// Lowercase hex SHA-256 of the FULL raw file content.
    pub checksum: String,
    /// Authored date from the manifest entry.
    pub created: Option<String>,
    /// Author from the manifest entry.
    pub author: Option<String>,
    pub why: Option<String>,
    /// Raw file content (from the same read that produced `checksum`).
    raw: String,
}

impl Migration {
    /// Build a Migration from a single atomic file read.
    /// `checksum` and `raw` must come from the same `read_and_checksum` call.
    pub(crate) fn from_metadata(
        version: u32,
        file: &str,
        checksum: String,
        raw: String,
        created: Option<&str>,
        author: Option<&str>,
        why: Option<&str>,
    ) -> Self {
        let name = file.strip_suffix(".sql").unwrap_or(file).to_owned();
        Self {
            version,
            file: file.to_owned(),
            name,
            checksum,
            raw,
            created: created.map(str::to_owned),
            author: author.map(str::to_owned),
            why: why.map(str::to_owned),
        }
    }

    /// Return the UP SQL section from the stored raw content.
    pub(crate) fn read_up(&self) -> (String, Option<String>) {
        split_up_down(&self.raw)
    }

    /// Return the DOWN SQL section from the stored raw content.
    pub(crate) fn read_down(&self) -> Option<String> {
        split_up_down(&self.raw).1
    }

    /// The UP section of this migration's SQL (everything before `-- DOWN ==`).
    ///
    /// Returns an owned `String` because the split is computed on demand from the
    /// stored raw content; there is no pre-split buffer to borrow from.
    pub fn up(&self) -> String {
        split_up_down(&self.raw).0
    }

    /// The DOWN section of this migration's SQL (everything after `-- DOWN ==`),
    /// or `None` if the file has no DOWN section.
    pub fn down(&self) -> Option<String> {
        split_up_down(&self.raw).1
    }
}

/// A file from `00_setup/`, run unconditionally and untracked.
/// Content is loaded lazily at run time.
#[derive(Debug)]
pub struct SetupFile {
    pub name: String,
    pub(crate) path: PathBuf,
}

impl SetupFile {
    pub(crate) fn read_sql(&self) -> Result<String> {
        Ok(std::fs::read_to_string(&self.path)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn write_temp(content: &str) -> (tempfile::NamedTempFile, std::path::PathBuf) {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        let p = f.path().to_path_buf();
        (f, p)
    }

    #[test]
    fn split_with_down() {
        let raw = "CREATE TABLE t (id INT);\n-- DOWN ==\nDROP TABLE t;";
        let (up, down) = split_up_down(raw);
        assert_eq!(up, "CREATE TABLE t (id INT);");
        assert_eq!(down.as_deref(), Some("DROP TABLE t;"));
    }

    #[test]
    fn split_without_down() {
        let raw = "CREATE TABLE t (id INT);";
        let (up, down) = split_up_down(raw);
        assert_eq!(up, "CREATE TABLE t (id INT);");
        assert!(down.is_none());
    }

    #[test]
    fn separator_must_be_exact_trim() {
        // Extra text after separator => NOT treated as separator
        let raw = "UP;\n-- DOWN == extra\nDOWN;";
        let (_, down) = split_up_down(raw);
        assert!(down.is_none(), "should not split on partial separator");
    }

    #[test]
    fn checksum_is_of_full_file() {
        use sha2::Digest;
        let raw = "CREATE TABLE t (id INT);\n-- DOWN ==\nDROP TABLE t;";
        let (_tmp, path) = write_temp(raw);
        let (got_checksum, got_raw) = read_and_checksum(&path).unwrap();
        assert_eq!(got_raw, raw);

        let mut h = Sha256::new();
        h.update(raw.as_bytes());
        let expected: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(got_checksum, expected);
    }

    #[test]
    fn read_up_and_down_from_migration() {
        let raw = "CREATE TABLE t (id INT);\n-- DOWN ==\nDROP TABLE t;";
        let (_tmp, path) = write_temp(raw);
        let (checksum, file_raw) = read_and_checksum(&path).unwrap();
        let m = Migration::from_metadata(1, "20260101_01_init.sql", checksum, file_raw, None, None, None);
        drop(path); // no longer needed after read_and_checksum
        let (up, down) = m.read_up();
        assert_eq!(up, "CREATE TABLE t (id INT);");
        assert_eq!(down.as_deref(), Some("DROP TABLE t;"));
        assert_eq!(m.read_down().as_deref(), Some("DROP TABLE t;"));
    }

    #[test]
    fn read_up_without_down() {
        let raw = "CREATE TABLE t (id INT);";
        let (_tmp, path) = write_temp(raw);
        let (checksum, file_raw) = read_and_checksum(&path).unwrap();
        let m = Migration::from_metadata(1, "20260101_01_init.sql", checksum, file_raw, None, None, None);
        drop(path); // no longer needed after read_and_checksum
        let (up, down) = m.read_up();
        assert_eq!(up, "CREATE TABLE t (id INT);");
        assert!(down.is_none());
        assert!(m.read_down().is_none());
    }

    #[test]
    fn public_up_and_down_with_separator() {
        let raw = "CREATE TABLE t (id INT);\n-- DOWN ==\nDROP TABLE t;";
        let (_tmp, path) = write_temp(raw);
        let (checksum, file_raw) = read_and_checksum(&path).unwrap();
        let m = Migration::from_metadata(1, "20260101_01_init.sql", checksum, file_raw, None, None, None);
        assert_eq!(m.up(), "CREATE TABLE t (id INT);");
        assert_eq!(m.down().as_deref(), Some("DROP TABLE t;"));
    }

    #[test]
    fn public_up_and_down_without_separator() {
        let raw = "CREATE TABLE t (id INT);";
        let (_tmp, path) = write_temp(raw);
        let (checksum, file_raw) = read_and_checksum(&path).unwrap();
        let m = Migration::from_metadata(1, "20260101_01_init.sql", checksum, file_raw, None, None, None);
        assert_eq!(m.up(), "CREATE TABLE t (id INT);");
        assert!(m.down().is_none());
    }
}
