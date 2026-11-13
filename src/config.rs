use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

/// Personal pantry: skills, rules, extensions.
///
/// Order: `$CHEF_KITCHEN`, `~/kitchen`, walk up from cwd.
pub fn pantry_root() -> PathBuf {
    if let Ok(raw) = std::env::var("CHEF_KITCHEN") {
        return expand_path(&raw);
    }
    let home = expand_path("~/kitchen");
    if is_pantry_root(&home) {
        return home;
    }
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(found) = find_pantry(&cwd) {
            return found;
        }
    }
    let tool = tool_root();
    if is_pantry_root(&tool) {
        return tool;
    }
    expand_path("~/kitchen")
}

/// Hint for the user when the kitchen has no `skills/` folder; `None` when it does.
pub fn missing_pantry_hint() -> Option<String> {
    let root = pantry_root();
    if is_pantry_root(&root) {
        return None;
    }
    Some(format!(
        "no kitchen at {}: create a skills/ folder there or set CHEF_KITCHEN",
        root.display()
    ))
}

/// Where `chef bootstrap` writes its embedded skills before linking them.
pub fn bootstrap_skills_dir() -> PathBuf {
    expand_path("~/.chef/bootstrap/skills")
}

/// chef CLI repo (harness adapters, built-in tool paths).
pub fn tool_root() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(found) = find_tool_repo(&cwd) {
            return found;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_pantry(start: &Path) -> Option<PathBuf> {
    for dir in start.ancestors() {
        if is_pantry_root(dir) {
            return Some(dir.to_path_buf());
        }
        let nested = dir.join("kitchen");
        if is_pantry_root(&nested) {
            return Some(nested);
        }
    }
    None
}

fn find_tool_repo(start: &Path) -> Option<PathBuf> {
    for dir in start.ancestors() {
        if is_tool_repo(dir) {
            return Some(dir.to_path_buf());
        }
    }
    None
}

fn is_pantry_root(dir: &Path) -> bool {
    dir.join("skills").is_dir()
}

/// True for a checkout of this repository (what `chef update` builds from).
pub fn is_tool_repo(dir: &Path) -> bool {
    dir.join("Cargo.toml").is_file()
        && dir.join("harnesses/targets.json").is_file()
        && dir.join("src/lib.rs").is_file()
}

#[derive(Debug, Clone, Deserialize)]
struct TargetsFile {
    harnesses: Vec<Harness>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Harness {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub project: bool,
    pub kinds: BTreeMap<String, KindSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KindSpec {
    pub dir: String,
    pub mode: String,
    pub dest_ext: Option<String>,
}

impl Harness {
    pub fn dest_dir(&self, kind: &str, project: Option<&Path>) -> Result<PathBuf> {
        let spec = self
            .kinds
            .get(kind)
            .with_context(|| format!("harness {} has no kind {kind}", self.id))?;
        if self.project {
            let project = project.context("project path required")?;
            Ok(project.join(&spec.dir))
        } else {
            Ok(expand_path(&spec.dir))
        }
    }

    pub fn kind_names(&self) -> String {
        self.kinds.keys().cloned().collect::<Vec<_>>().join(", ")
    }
}

/// Harness definitions are static CLI metadata, so they're embedded at compile
/// time rather than read from disk. Reading them from `tool_root()` at runtime
/// broke every harness-aware command once a binary built from a temporary
/// checkout (e.g. `cargo install --path /tmp/chef-checkout`) outlived that
/// checkout.
const TARGETS_JSON: &str = include_str!("../harnesses/targets.json");

pub fn load_harnesses() -> Result<Vec<Harness>> {
    let file: TargetsFile = serde_json::from_str(TARGETS_JSON).context("parse targets.json")?;
    let mut harnesses = file.harnesses;
    sort_harnesses(&mut harnesses);
    Ok(harnesses)
}

/// Group harnesses by family (id up to the first `-`, like `claude-user` and
/// `claude-project` under `claude`), user-level before project-level inside
/// each family. Families follow their first appearance in targets.json.
fn sort_harnesses(harnesses: &mut [Harness]) {
    let group_of = |id: &str| id.split('-').next().unwrap_or(id).to_string();
    let mut groups: Vec<String> = Vec::new();
    for h in harnesses.iter() {
        let group = group_of(&h.id);
        if !groups.contains(&group) {
            groups.push(group);
        }
    }
    harnesses.sort_by_key(|h| {
        let pos = groups
            .iter()
            .position(|g| *g == group_of(&h.id))
            .unwrap_or(usize::MAX);
        (pos, h.project)
    });
}

pub fn harness_by_id(id: &str) -> Result<Harness> {
    load_harnesses()?
        .into_iter()
        .find(|h| h.id == id)
        .with_context(|| format!("unknown harness: {id}"))
}

pub fn expand_path(raw: &str) -> PathBuf {
    if raw == "~" {
        return home_dir();
    }
    if let Some(rest) = raw.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    PathBuf::from(raw)
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .expect("HOME is unset")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilde_expands() {
        let got = expand_path("~/.cursor/skills");
        assert!(got.ends_with(".cursor/skills"));
        assert!(!got.to_string_lossy().contains('~'));
    }

    #[test]
    fn is_pantry_root_checks_skills_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!is_pantry_root(tmp.path()));
        std::fs::create_dir(tmp.path().join("skills")).unwrap();
        assert!(is_pantry_root(tmp.path()));
    }
}
