use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::config::{expand_path, pantry_root, Harness, KindSpec};

const KIND_ORDER: [&str; 3] = ["skills", "rules", "extensions"];
const EXT_SUFFIXES: [&str; 5] = [".ts", ".js", ".mjs", ".py", ".sh"];

#[derive(Debug, Clone)]
pub struct Item {
    pub kind: String,
    pub name: String,
    pub source: PathBuf,
    pub dest: PathBuf,
}

impl Item {
    pub fn key(&self) -> String {
        format!("{}/{}", self.kind, self.name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    Missing,
    Linked,
    Foreign,
    Blocked,
}

impl LinkState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Linked => "linked",
            Self::Foreign => "foreign",
            Self::Blocked => "blocked",
        }
    }

    pub fn selectable(self) -> bool {
        matches!(self, Self::Missing | Self::Linked)
    }
}

impl std::fmt::Display for LinkState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

pub fn items_for(harness: &Harness, project: Option<&Path>) -> Result<Vec<Item>> {
    if harness.project && project.is_none() {
        bail!("project path required for {}", harness.id);
    }
    let mut out = Vec::new();
    for kind in KIND_ORDER {
        let Some(spec) = harness.kinds.get(kind) else {
            continue;
        };
        let dest_root = harness.dest_dir(kind, project)?;
        for source in iter_sources(kind)? {
            let dest = dest_root.join(dest_name(&source, spec));
            out.push(Item {
                kind: kind.to_string(),
                name: file_name(&source),
                source: source.canonicalize().unwrap_or(source),
                dest,
            });
        }
    }
    Ok(out)
}

pub fn list_names(kind: &str) -> Result<Vec<String>> {
    Ok(list_kind(kind)?.into_iter().map(|e| e.name).collect())
}

#[derive(Debug, Clone)]
pub struct PantryEntry {
    pub name: String,
    pub preview: PathBuf,
    /// Skill dir, rule file, or extension file.
    pub root: PathBuf,
    /// Where this entry was found (`kitchen`, `cursor-user`, …).
    pub location: String,
}

/// Repo-only listing (used by `kitchen serve`).
pub fn list_kind(kind: &str) -> Result<Vec<PantryEntry>> {
    Ok(iter_sources(kind)?
        .into_iter()
        .map(|p| entry_from_path(kind, "pantry", &p))
        .collect())
}

/// Pantry view. `all_sources`: repo only, or repo + harness dirs on this machine.
pub fn list_pantry(kind: &str, all_sources: bool) -> Result<Vec<PantryEntry>> {
    let roots = if all_sources {
        pantry_roots(kind)
    } else {
        vec![("pantry", pantry_root().join(kind))]
    };
    let mut raw = Vec::new();
    for (label, root) in roots {
        for path in iter_at_root(kind, &root)? {
            raw.push((label.to_string(), path));
        }
    }
    Ok(dedupe_entries(kind, raw))
}

/// Collapse symlinked duplicates (e.g. repo skill + `cursor-user` link to same dir).
fn dedupe_entries(kind: &str, raw: Vec<(String, PathBuf)>) -> Vec<PantryEntry> {
    let mut by_key: BTreeMap<String, PantryEntry> = BTreeMap::new();
    for (label, path) in raw {
        let entry = entry_from_path(kind, &label, &path);
        let key = pantry_canonical_key(&path);
        match by_key.get_mut(&key) {
            Some(existing) => merge_entry_location(existing, &entry.location),
            None => {
                by_key.insert(key, entry);
            }
        }
    }
    let mut out: Vec<_> = by_key.into_values().collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn pantry_canonical_key(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

/// True when this entry lives in the personal pantry (may also be linked elsewhere).
pub fn is_kitchen_entry(entry: &PantryEntry) -> bool {
    entry
        .location
        .split(" · ")
        .any(|loc| loc == "pantry")
}

fn merge_entry_location(existing: &mut PantryEntry, new_loc: &str) {
    if existing
        .location
        .split(" · ")
        .any(|loc| loc == new_loc)
    {
        return;
    }
    existing.location = format!("{} · {}", existing.location, new_loc);
}

fn entry_from_path(kind: &str, location: &str, path: &Path) -> PantryEntry {
    PantryEntry {
        name: pantry_name(kind, path),
        preview: preview_path(kind, path),
        root: path.to_path_buf(),
        location: location.to_string(),
    }
}

fn pantry_roots(kind: &str) -> Vec<(&'static str, PathBuf)> {
    let root = pantry_root();
    let mut out = vec![("pantry", root.join(kind))];
    match kind {
        "skills" => {
            out.push(("cursor-user", expand_path("~/.cursor/skills")));
            out.push(("claude-user", expand_path("~/.claude/skills")));
            if let Ok(cwd) = std::env::current_dir() {
                out.push(("cursor-project", cwd.join(".cursor/skills")));
                out.push(("claude-project", cwd.join(".claude/skills")));
            }
        }
        "rules" => {
            out.push(("cursor-user", expand_path("~/.cursor/rules")));
            if let Ok(cwd) = std::env::current_dir() {
                out.push(("cursor-project", cwd.join(".cursor/rules")));
            }
        }
        "extensions" => {
            out.push(("pi-user", expand_path("~/.pi/agent/extensions")));
        }
        _ => {}
    }
    out
}

pub fn classify(item: &Item) -> LinkState {
    match fs::symlink_metadata(&item.dest) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => LinkState::Missing,
        Err(_) => LinkState::Blocked,
        Ok(meta) if meta.file_type().is_symlink() => match fs::canonicalize(&item.dest) {
            Ok(resolved) if resolved == item.source => LinkState::Linked,
            _ => LinkState::Foreign,
        },
        Ok(_) => LinkState::Blocked,
    }
}

fn iter_sources(kind: &str) -> Result<Vec<PathBuf>> {
    iter_at_root(kind, &pantry_root().join(kind))
}

fn iter_at_root(kind: &str, root: &Path) -> Result<Vec<PathBuf>> {
    if is_cursor_builtin(root) || !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<PathBuf> = match kind {
        "skills" => fs::read_dir(root)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir() && !should_skip(p) && p.join("SKILL.md").is_file())
            .collect(),
        "rules" => fs::read_dir(root)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && is_rule_file(p) && !should_skip(p))
            .collect(),
        "extensions" => fs::read_dir(root)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && !should_skip(p) && has_ext_suffix(p))
            .collect(),
        _ => Vec::new(),
    };
    paths.sort();
    Ok(paths)
}

fn is_rule_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md") | Some("mdc")
    )
}

fn is_cursor_builtin(root: &Path) -> bool {
    root.to_string_lossy().contains("skills-cursor")
}

fn preview_path(kind: &str, source: &Path) -> PathBuf {
    if kind == "skills" {
        source.join("SKILL.md")
    } else {
        source.to_path_buf()
    }
}

fn dest_name(source: &Path, spec: &KindSpec) -> String {
    if spec.mode == "dir" {
        return file_name(source);
    }
    if let Some(ext) = &spec.dest_ext {
        return format!("{}{ext}", stem(source));
    }
    file_name(source)
}

fn should_skip(path: &Path) -> bool {
    let name = file_name(path);
    name.starts_with('_') || matches!(name.to_ascii_lowercase().as_str(), "readme.md" | ".gitkeep")
}

fn has_ext_suffix(path: &Path) -> bool {
    let name = file_name(path);
    EXT_SUFFIXES.iter().any(|s| name.ends_with(s))
}

fn pantry_name(kind: &str, path: &Path) -> String {
    if kind == "rules" {
        return stem(path);
    }
    file_name(path)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| file_name(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_rule_gets_mdc() {
        let spec = KindSpec {
            dir: "~/.cursor/rules".into(),
            mode: "file".into(),
            dest_ext: Some(".mdc".into()),
        };
        assert_eq!(
            dest_name(Path::new("rules/prefer-edit.md"), &spec),
            "prefer-edit.mdc"
        );
    }

    #[test]
    fn rule_pantry_name_strips_md() {
        assert_eq!(
            pantry_name("rules", Path::new("rules/prefer-edit-over-write.md")),
            "prefer-edit-over-write"
        );
        assert_eq!(
            pantry_name("skills", Path::new("skills/local-pi-qwen")),
            "local-pi-qwen"
        );
    }

    #[test]
    fn skill_preview_is_skill_md() {
        assert_eq!(
            preview_path("skills", Path::new("skills/local-pi-qwen")),
            Path::new("skills/local-pi-qwen/SKILL.md")
        );
        assert_eq!(
            preview_path("rules", Path::new("rules/foo.md")),
            Path::new("rules/foo.md")
        );
    }

    #[test]
    fn is_kitchen_entry_detects_repo_and_external() {
        let repo = PantryEntry {
            name: "foo".into(),
            preview: PathBuf::from("skills/foo/SKILL.md"),
            root: PathBuf::from("skills/foo"),
            location: "pantry".into(),
        };
        let linked = PantryEntry {
            location: "pantry · cursor-user".into(),
            ..repo.clone()
        };
        let external = PantryEntry {
            location: "cursor-user".into(),
            ..repo.clone()
        };
        assert!(is_kitchen_entry(&repo));
        assert!(is_kitchen_entry(&linked));
        assert!(!is_kitchen_entry(&external));
    }

    #[test]
    fn dedupe_entries_merges_symlinked_skill() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo/foo");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("SKILL.md"), "body").unwrap();
        let cursor = tmp.path().join("cursor");
        fs::create_dir(&cursor).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&repo, cursor.join("foo")).unwrap();
        #[cfg(not(unix))]
        return;

        let raw = vec![
            ("pantry".into(), repo.clone()),
            ("cursor-user".into(), cursor.join("foo")),
        ];
        let out = dedupe_entries("skills", raw);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "foo");
        assert_eq!(out[0].location, "pantry · cursor-user");
    }

    #[test]
    fn is_rule_file_accepts_md_and_mdc() {
        assert!(is_rule_file(Path::new("rules/foo.md")));
        assert!(is_rule_file(Path::new("rules/foo.mdc")));
        assert!(!is_rule_file(Path::new("rules/foo.txt")));
    }

    #[test]
    fn skip_template_and_readme() {
        assert!(should_skip(Path::new("skills/_template")));
        assert!(should_skip(Path::new("rules/README.md")));
        assert!(!should_skip(Path::new("rules/prefer-edit.md")));
    }
}
