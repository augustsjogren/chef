use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::config::{repo_root, Harness, KindSpec};

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
    Ok(iter_sources(kind)?
        .into_iter()
        .map(|p| file_name(&p))
        .collect())
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
    let root = repo_root().join(kind);
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<PathBuf> = match kind {
        "skills" => fs::read_dir(&root)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir() && !should_skip(p) && p.join("SKILL.md").is_file())
            .collect(),
        "rules" => fs::read_dir(&root)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.extension().and_then(|e| e.to_str()) == Some("md")
                    && !should_skip(p)
            })
            .collect(),
        "extensions" => fs::read_dir(&root)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && !should_skip(p) && has_ext_suffix(p))
            .collect(),
        _ => Vec::new(),
    };
    paths.sort();
    Ok(paths)
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
    fn skip_template_and_readme() {
        assert!(should_skip(Path::new("skills/_template")));
        assert!(should_skip(Path::new("rules/README.md")));
        assert!(!should_skip(Path::new("rules/prefer-edit.md")));
    }
}
