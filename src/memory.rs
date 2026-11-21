//! Managed `@import` blocks in harness memory files (e.g. `~/.claude/CLAUDE.md`).
//!
//! Claude Code loads `CLAUDE.md` but has no rules directory of its own, so
//! `chef serve` symlinks kitchen rules into e.g. `~/.claude/rules/` and
//! rewrites a marker-delimited block of `@path` imports in the memory file.
//! Everything outside the markers is left alone.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::{expand_path, Harness};
use crate::item::{classify, items_for, Item, LinkState};

pub const BEGIN_MARKER: &str = "<!-- chef:begin -->";
pub const END_MARKER: &str = "<!-- chef:end -->";

/// Sync the managed block for `harness`'s rules kind, if it declares a
/// `memory` file. Returns a report for callers to display: the CLI prints
/// it, the TUI adds it to the flash line.
///
/// The block lists every rule linked (or about to be, when `link_pass`)
/// into the kitchen, so it stays correct after both `link` and `unlink`.
pub fn sync_memory(
    harness: &Harness,
    project: Option<&Path>,
    dry_run: bool,
    link_pass: bool,
) -> Result<Option<MemoryReport>> {
    let Some(memory) = harness.kinds.get("rules").and_then(|s| s.memory.as_deref()) else {
        return Ok(None);
    };
    let memory_path = memory_path(memory, project)?;
    let imports = linked_rule_imports(harness, project, link_pass)?;

    let action = plan(&memory_path, &imports);
    match (action, dry_run) {
        (Action::None, _) => {}
        (Action::Write, false) => write_block(&memory_path, &imports)?,
        (Action::Remove, false) => remove_block(&memory_path)?,
        (_, true) => {}
    }
    Ok(Some(MemoryReport {
        path: memory_path,
        action,
        imports: imports.len(),
    }))
}

/// What a `sync_memory` pass did to a memory file.
#[derive(Debug)]
pub struct MemoryReport {
    pub path: PathBuf,
    pub action: Action,
    pub imports: usize,
}

impl MemoryReport {
    pub fn line(&self, dry_run: bool) -> String {
        match (self.action, dry_run) {
            (Action::None, _) => format!("memory: {} (up to date)", self.path.display()),
            (action, true) => format!("memory: would {} {}", action, self.path.display()),
            (Action::Write, false) => {
                format!("memory: {} ({})", self.path.display(), self.imports)
            }
            (Action::Remove, false) => format!("memory: {} (removed)", self.path.display()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Write,
    Remove,
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("leave"),
            Self::Write => f.write_str("update"),
            Self::Remove => f.write_str("remove"),
        }
    }
}

fn plan(memory_path: &Path, imports: &[String]) -> Action {
    let existing = fs::read_to_string(memory_path).unwrap_or_default();
    let expected = block(imports);
    match (imports.is_empty(), extract_block(&existing)) {
        (true, Some(_)) => Action::Remove,
        (true, None) => Action::None,
        (false, None) => Action::Write,
        (false, Some(current)) if current == expected => Action::None,
        (false, Some(_)) => Action::Write,
    }
}

/// Import lines for every rule that is (or, on a link pass, would be)
/// symlinked from the kitchen; unlinked rules drop out of the block.
fn linked_rule_imports(
    harness: &Harness,
    project: Option<&Path>,
    link_pass: bool,
) -> Result<Vec<String>> {
    let mut items: Vec<Item> = items_for(harness, project)?
        .into_iter()
        .filter(|item| item.kind == "rules")
        .filter(|item| match classify(item) {
            LinkState::Linked | LinkState::Foreign => true,
            LinkState::Missing => link_pass,
            LinkState::Blocked => false,
        })
        .collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(items.iter().map(|item| format!("@{}", item.dest.display())).collect())
}

fn memory_path(memory: &str, project: Option<&Path>) -> Result<PathBuf> {
    if memory.starts_with('~') {
        Ok(expand_path(memory))
    } else {
        let project = project.context("memory file is relative; needs --project")?;
        Ok(project.join(memory))
    }
}

/// The managed block, markers included; empty when there are no imports.
fn block(imports: &[String]) -> String {
    if imports.is_empty() {
        return String::new();
    }
    let mut out = String::from(BEGIN_MARKER);
    out.push('\n');
    for line in imports {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(END_MARKER);
    out.push('\n');
    out
}

fn extract_block(content: &str) -> Option<String> {
    let start = content.find(BEGIN_MARKER)?;
    let end_off = content[start..].find(END_MARKER)?;
    Some(content[start..start + end_off + END_MARKER.len()].to_string() + "\n")
}

fn write_block(memory_path: &Path, imports: &[String]) -> Result<()> {
    let existing = fs::read_to_string(memory_path).unwrap_or_default();
    let block = block(imports);
    let content = match existing.find(BEGIN_MARKER) {
        Some(start) => {
            let end_off = existing[start..]
                .find(END_MARKER)
                .with_context(|| format!("{} has {} without {}", memory_path.display(), BEGIN_MARKER, END_MARKER))?;
            let tail = &existing[start + end_off + END_MARKER.len()..];
            join(&existing[..start], &block, tail)
        }
        None => join(&existing, &block, ""),
    };
    if let Some(parent) = memory_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    fs::write(memory_path, content).with_context(|| format!("write {}", memory_path.display()))?;
    Ok(())
}

fn remove_block(memory_path: &Path) -> Result<()> {
    let existing = fs::read_to_string(memory_path).with_context(|| format!("read {}", memory_path.display()))?;
    let content = match extract_block(&existing) {
        Some(block) => {
            let start = existing.find(BEGIN_MARKER).unwrap_or_default();
            let end = start + block.len() - 1;
            join(&existing[..start], "", &existing[end..])
        }
        None => existing,
    };
    fs::write(memory_path, content).with_context(|| format!("write {}", memory_path.display()))?;
    Ok(())
}

/// Splice head + block + tail with tidy blank-line separation.
fn join(head: &str, block: &str, tail: &str) -> String {
    let head = head.trim();
    let tail = tail.trim();
    let block = block.trim_end();
    let mut parts: Vec<&str> = Vec::new();
    if !head.is_empty() {
        parts.push(head);
    }
    if !block.is_empty() {
        parts.push(block);
    }
    if !tail.is_empty() {
        parts.push(tail);
    }
    let mut out = parts.join("\n\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn imports(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| format!("@/r/{}", n)).collect()
    }

    #[test]
    fn plan_write_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        assert_eq!(plan(&path, &imports(&["conduct.md"])), Action::Write);
    }

    #[test]
    fn plan_none_when_current() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        write_block(&path, &imports(&["conduct.md"])).unwrap();
        assert_eq!(plan(&path, &imports(&["conduct.md"])), Action::None);
    }

    #[test]
    fn plan_stays_none_with_manual_content() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        fs::write(&path, "manual note\n").unwrap();
        write_block(&path, &imports(&["conduct.md"])).unwrap();
        write_block(&path, &imports(&["conduct.md"])).unwrap();
        assert_eq!(plan(&path, &imports(&["conduct.md"])), Action::None);
    }

    #[test]
    fn plan_remove_when_block_and_no_rules() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        write_block(&path, &imports(&["conduct.md"])).unwrap();
        assert_eq!(plan(&path, &[]), Action::Remove);
    }

    #[test]
    fn write_then_remove_preserves_manual_content() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        fs::write(&path, "manual note\n").unwrap();
        write_block(&path, &imports(&["a.md", "b.md"])).unwrap();
        let after = fs::read_to_string(&path).unwrap();
        assert!(after.starts_with("manual note\n\n"));
        assert!(after.contains("@/r/a.md\n@/r/b.md"));
        assert!(after.ends_with(&format!("{}\n", END_MARKER)));

        remove_block(&path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "manual note\n");
    }

    #[test]
    fn rewrite_block_replaces_only_block() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        write_block(&path, &imports(&["a.md"])).unwrap();
        write_block(&path, &imports(&["b.md"])).unwrap();
        let after = fs::read_to_string(&path).unwrap();
        assert!(!after.contains("@/r/a.md"));
        assert!(after.contains("@/r/b.md"));
        assert_eq!(after.matches(BEGIN_MARKER).count(), 1);
    }

    #[test]
    fn remove_without_block_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("CLAUDE.md");
        fs::write(&path, "note\n").unwrap();
        remove_block(&path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "note\n");
    }
}
