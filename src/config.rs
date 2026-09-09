use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

/// Repo with skills/rules/kitchen data.
///
/// Order: `$KITCHEN_ROOT`, walk up from cwd, then the tree this binary was built from.
pub fn repo_root() -> PathBuf {
    if let Ok(raw) = std::env::var("KITCHEN_ROOT") {
        return expand_path(&raw);
    }
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(found) = find_repo(&cwd) {
            return found;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_repo(start: &Path) -> Option<PathBuf> {
    for dir in start.ancestors() {
        if is_kitchen_repo(dir) {
            return Some(dir.to_path_buf());
        }
    }
    None
}

fn is_kitchen_repo(dir: &Path) -> bool {
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

pub fn load_harnesses() -> Result<Vec<Harness>> {
    let path = repo_root().join("harnesses/targets.json");
    let raw = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let file: TargetsFile = serde_json::from_str(&raw).context("parse targets.json")?;
    Ok(file.harnesses)
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
}
