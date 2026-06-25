/// Agent-rules writing logic for `soma-schema init`.
///
/// Writes the canonical soma-schema agent-rules block to the appropriate file(s)
/// in the current directory. Appends idempotently when the target file already
/// exists (skips if a "soma-schema" migrations section is already present; never
/// clobbers).
use std::path::Path;

use crate::error::Result;

// ── Embedded content ──────────────────────────────────────────────────────────

/// Canonical agent-rules block sourced from the repo-root AGENTS.md.
/// Agents are NON-DETERMINISTIC; soma-schema gives them rules and guardrails
/// that catch mistakes — not correctness guarantees.
pub const AGENTS_RULES_TEXT: &str = include_str!("../AGENTS.md");

/// Claude skill content for `~/.claude/skills/soma-schema/SKILL.md`.
pub const SKILL_TEXT: &str = include_str!("../assets/soma-schema-skill.md");

// ── Sentinel for idempotency ──────────────────────────────────────────────────

/// Substring checked to decide whether the section is already present.
const IDEMPOTENCY_SENTINEL: &str = "soma-schema";
const SECTION_HEADER: &str = "\n\n---\n\n";

// ── Public types ──────────────────────────────────────────────────────────────

/// Which agent-rules file(s) `init --rules` should write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RulesTarget {
    /// `AGENTS.md` (default — works with Claude Code, Codex, Cursor, Zed, …)
    Agents,
    /// `CLAUDE.md`
    Claude,
    /// `.cursor/rules/soma-schema.mdc`
    Cursor,
    /// `.windsurf/rules/soma-schema.md`
    Windsurf,
    /// All of the above
    All,
    /// Skip rules writing entirely
    None,
}

impl std::fmt::Display for RulesTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RulesTarget::Agents => write!(f, "agents"),
            RulesTarget::Claude => write!(f, "claude"),
            RulesTarget::Cursor => write!(f, "cursor"),
            RulesTarget::Windsurf => write!(f, "windsurf"),
            RulesTarget::All => write!(f, "all"),
            RulesTarget::None => write!(f, "none"),
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Write the requested agent-rules file(s) into `cwd`.
///
/// Returns a list of short messages — one per file — describing what happened.
pub fn write_rules(cwd: &Path, target: &RulesTarget) -> Result<Vec<String>> {
    let files = target_files(target);
    let mut written = Vec::new();
    for rel_path in files {
        let path = cwd.join(&rel_path);
        let msg = write_rules_file(&path, AGENTS_RULES_TEXT)?;
        written.push(format!("{rel_path}: {msg}"));
    }
    Ok(written)
}

/// Install the Claude skill to `~/.claude/skills/soma-schema/SKILL.md`.
///
/// Returns a short message describing what happened.
pub fn install_skill() -> Result<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
    let skill_dir = Path::new(&home)
        .join(".claude")
        .join("skills")
        .join("soma-schema");
    std::fs::create_dir_all(&skill_dir)?;
    let skill_path = skill_dir.join("SKILL.md");
    let msg = write_rules_file(&skill_path, SKILL_TEXT)?;
    Ok(format!("{}: {msg}", skill_path.display()))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn target_files(target: &RulesTarget) -> Vec<String> {
    match target {
        RulesTarget::Agents => vec!["AGENTS.md".into()],
        RulesTarget::Claude => vec!["CLAUDE.md".into()],
        RulesTarget::Cursor => vec![".cursor/rules/soma-schema.mdc".into()],
        RulesTarget::Windsurf => vec![".windsurf/rules/soma-schema.md".into()],
        RulesTarget::All => vec![
            "AGENTS.md".into(),
            "CLAUDE.md".into(),
            ".cursor/rules/soma-schema.mdc".into(),
            ".windsurf/rules/soma-schema.md".into(),
        ],
        RulesTarget::None => vec![],
    }
}

/// Write `content` to `path` idempotently.
/// - File absent → create.
/// - File present and already contains sentinel → skip (return "skipped").
/// - File present but no sentinel → append (return "appended …").
fn write_rules_file(path: &Path, content: &str) -> Result<String> {
    if path.exists() {
        let existing = std::fs::read_to_string(path)?;
        if existing.contains(IDEMPOTENCY_SENTINEL) {
            return Ok("already contains soma-schema section — skipped".into());
        }
        let appended = format!("{existing}{SECTION_HEADER}{content}");
        std::fs::write(path, &appended)?;
        return Ok("appended soma-schema section".into());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok("created".into())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents_target_creates_agents_md() {
        let dir = tempfile::tempdir().unwrap();
        let msgs = write_rules(dir.path(), &RulesTarget::Agents).unwrap();
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].contains("AGENTS.md"), "path: {}", msgs[0]);
        assert!(msgs[0].contains("created"), "action: {}", msgs[0]);
        let content = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
        assert!(content.contains(IDEMPOTENCY_SENTINEL));
    }

    #[test]
    fn claude_target_creates_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        let msgs = write_rules(dir.path(), &RulesTarget::Claude).unwrap();
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].contains("CLAUDE.md"), "path: {}", msgs[0]);
        let content = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(content.contains(IDEMPOTENCY_SENTINEL));
    }

    #[test]
    fn none_target_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let msgs = write_rules(dir.path(), &RulesTarget::None).unwrap();
        assert!(msgs.is_empty());
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .collect::<std::result::Result<_, _>>()
            .unwrap();
        assert!(entries.is_empty(), "expected no files written");
    }

    #[test]
    fn idempotent_when_section_already_present() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        std::fs::write(
            &path,
            "# existing\n\nThis mentions soma-schema migrations.\n",
        )
        .unwrap();
        let msgs = write_rules(dir.path(), &RulesTarget::Agents).unwrap();
        assert!(
            msgs[0].contains("skipped"),
            "expected skipped, got: {}",
            msgs[0]
        );
        let after = std::fs::read_to_string(&path).unwrap();
        // Original content preserved and no extra blob appended.
        assert!(after.starts_with("# existing"));
        // The rule text (e.g. DOWN ==) should NOT have been appended.
        assert!(
            !after.contains("-- DOWN =="),
            "should not have appended rules"
        );
    }

    #[test]
    fn appends_when_file_has_no_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        std::fs::write(&path, "# My project CLAUDE.md\n\nSome instructions.\n").unwrap();
        let msgs = write_rules(dir.path(), &RulesTarget::Claude).unwrap();
        assert!(
            msgs[0].contains("appended"),
            "expected appended, got: {}",
            msgs[0]
        );
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("My project CLAUDE.md"),
            "original preserved"
        );
        assert!(content.contains(IDEMPOTENCY_SENTINEL), "rules appended");
    }

    #[test]
    fn all_target_writes_four_files() {
        let dir = tempfile::tempdir().unwrap();
        let msgs = write_rules(dir.path(), &RulesTarget::All).unwrap();
        assert_eq!(msgs.len(), 4);
        assert!(dir.path().join("AGENTS.md").exists());
        assert!(dir.path().join("CLAUDE.md").exists());
        assert!(dir.path().join(".cursor/rules/soma-schema.mdc").exists());
        assert!(dir.path().join(".windsurf/rules/soma-schema.md").exists());
    }

    #[test]
    fn cursor_and_windsurf_create_nested_dirs() {
        let dir = tempfile::tempdir().unwrap();
        write_rules(dir.path(), &RulesTarget::Cursor).unwrap();
        assert!(dir.path().join(".cursor/rules/soma-schema.mdc").exists());
        write_rules(dir.path(), &RulesTarget::Windsurf).unwrap();
        assert!(dir.path().join(".windsurf/rules/soma-schema.md").exists());
    }
}
