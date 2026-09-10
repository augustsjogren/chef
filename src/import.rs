use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::config::pantry_root;
use crate::item::{self, PantryEntry};

#[derive(Debug)]
pub struct ImportResult {
    pub repo_path: PathBuf,
    pub relinked: Option<String>,
}

/// Copy an external skill into `skills/<name>/`. By default replace the source with a symlink
/// to the repo copy so Cursor/Claude keep working without a second on-disk copy.
pub fn import_skill(
    name: &str,
    source: &Path,
    location: &str,
    keep_source: bool,
) -> Result<ImportResult> {
    if item::is_pantry_entry(&PantryEntry {
        name: name.into(),
        preview: source.join("SKILL.md"),
        location: location.into(),
        root: source.to_path_buf(),
    }) {
        bail!("skill already in kitchen");
    }
    if !source.join("SKILL.md").is_file() {
        bail!("not a skill directory: {}", source.display());
    }

    let dest = pantry_root().join("skills").join(name);
    if dest.exists() {
        bail!("skills/{name} already exists in kitchen");
    }

    copy_dir_all(source, &dest).with_context(|| format!("copy into {}", dest.display()))?;

    let relinked = if keep_source {
        None
    } else {
        replace_with_symlink(&dest, source)
            .with_context(|| format!("relink {}", source.display()))?;
        Some(location.to_string())
    };

    Ok(ImportResult {
        repo_path: dest,
        relinked,
    })
}

/// Find a skill that exists only outside kitchen.
pub fn find_external_skill(name: &str, from: Option<&str>) -> Result<(String, PathBuf)> {
    let mut matches: Vec<(String, PathBuf)> = item::list_pantry("skills", true)?
        .into_iter()
        .filter(|e| e.name == name && !item::is_pantry_entry(e))
        .filter(|e| from.is_none_or(|f| e.location == f))
        .map(|e| (e.location.clone(), e.root))
        .collect();

    if matches.is_empty() {
        bail!("external skill not found: {name}");
    }
    if matches.len() > 1 {
        let locs: Vec<_> = matches.iter().map(|(l, _)| l.as_str()).collect();
        bail!(
            "ambiguous skill {name} — found in {} (pass --from)",
            locs.join(", ")
        );
    }
    Ok(matches.remove(0))
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&path, &dest_path)?;
        } else {
            fs::copy(&path, dest_path)?;
        }
    }
    Ok(())
}

fn replace_with_symlink(target: &Path, link_path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(link_path)?;
    if meta.file_type().is_symlink() {
        fs::remove_file(link_path)?;
    } else if meta.is_dir() {
        fs::remove_dir_all(link_path)?;
    } else {
        fs::remove_file(link_path)?;
    }
    symlink(target, link_path)?;
    Ok(())
}

#[cfg(unix)]
fn symlink(source: &Path, dest: &Path) -> Result<()> {
    std::os::unix::fs::symlink(source, dest).map_err(Into::into)
}

#[cfg(not(unix))]
fn symlink(_source: &Path, _dest: &Path) -> Result<()> {
    bail!("import relink requires unix symlinks");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_copies_and_relinks_by_default() {
        #[cfg(not(unix))]
        return;

        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("pantry");
        fs::create_dir_all(repo.join("skills")).unwrap();
        let external_root = tmp.path().join("cursor");
        fs::create_dir_all(&external_root).unwrap();
        let external = external_root.join("my-skill");
        fs::create_dir_all(&external).unwrap();
        fs::write(external.join("SKILL.md"), "skill body").unwrap();
        fs::write(external.join("extra.md"), "more").unwrap();

        std::env::set_var("CHEF_KITCHEN", &repo);
        let dest = repo.join("skills/my-skill");
        import_skill("my-skill", &external, "cursor-user", false).unwrap();

        assert!(dest.join("SKILL.md").is_file());
        assert!(dest.join("extra.md").is_file());
        assert!(external.join("SKILL.md").is_file());
        assert_eq!(
            fs::read_to_string(external.join("SKILL.md")).unwrap(),
            "skill body"
        );
        assert!(fs::symlink_metadata(&external).unwrap().file_type().is_symlink());
        std::env::remove_var("CHEF_KITCHEN");
    }
}
