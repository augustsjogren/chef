use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::config::{missing_pantry_hint, pantry_root};
use crate::item::{self, PantryEntry};

#[derive(Debug)]
pub struct ImportResult {
    pub repo_path: PathBuf,
    pub relinked: Option<String>,
}

/// Copy an external skill into `skills/<name>`. Relinking the source is destructive and requires
/// `force`; use `keep_source` to retain the original directory instead.
pub fn import_skill(
    name: &str,
    source: &Path,
    location: &str,
    keep_source: bool,
    force: bool,
) -> Result<ImportResult> {
    if let Some(hint) = missing_pantry_hint() {
        bail!(hint);
    }
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

    if !keep_source && !force {
        bail!(
            "refusing to replace external source {}; rerun with --force or --keep-source",
            source.display()
        );
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
    let backup = backup_path(link_path)?;
    fs::rename(link_path, &backup)
        .with_context(|| format!("move {} to backup", link_path.display()))?;

    match symlink(target, link_path) {
        Ok(()) => {
            remove_path(&backup).with_context(|| format!("remove backup {}", backup.display()))?;
            Ok(())
        }
        Err(error) => {
            let _ = remove_path(link_path);
            fs::rename(&backup, link_path).with_context(|| {
                format!(
                    "restore {} after symlink failure ({error})",
                    link_path.display()
                )
            })?;
            Err(error)
        }
    }
}

fn backup_path(path: &Path) -> Result<PathBuf> {
    let parent = path.parent().context("path has no parent")?;
    let name = path
        .file_name()
        .context("path has no filename")?
        .to_string_lossy();
    for attempt in 0..100 {
        let candidate = parent.join(format!(
            ".{name}.chef-import-backup-{}-{attempt}",
            std::process::id()
        ));
        if fs::symlink_metadata(&candidate).is_err() {
            return Ok(candidate);
        }
    }
    bail!("could not allocate backup path for {}", path.display());
}

fn remove_path(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        fs::remove_file(path)?;
    } else {
        fs::remove_dir_all(path)?;
    }
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
    use std::sync::{Mutex, OnceLock};

    fn kitchen_env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    fn setup_import_fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("pantry");
        fs::create_dir_all(repo.join("skills")).unwrap();
        let external = tmp.path().join("cursor/my-skill");
        fs::create_dir_all(&external).unwrap();
        fs::write(external.join("SKILL.md"), "skill body").unwrap();
        fs::write(external.join("extra.md"), "more").unwrap();
        (tmp, repo, external)
    }

    #[test]
    fn import_copies_and_relinks_by_default() {
        #[cfg(not(unix))]
        return;

        let _lock = kitchen_env_lock();
        let (_tmp, repo, external) = setup_import_fixture();
        std::env::set_var("CHEF_KITCHEN", &repo);
        let dest = repo.join("skills/my-skill");
        import_skill("my-skill", &external, "cursor-user", false, true).unwrap();

        assert!(dest.join("SKILL.md").is_file());
        assert!(dest.join("extra.md").is_file());
        assert!(external.join("SKILL.md").is_file());
        assert_eq!(
            fs::read_to_string(external.join("SKILL.md")).unwrap(),
            "skill body"
        );
        assert!(fs::symlink_metadata(&external)
            .unwrap()
            .file_type()
            .is_symlink());
        std::env::remove_var("CHEF_KITCHEN");
    }

    #[test]
    fn import_refuses_relink_without_force_before_copying() {
        let _lock = kitchen_env_lock();
        let (_tmp, repo, external) = setup_import_fixture();
        std::env::set_var("CHEF_KITCHEN", &repo);

        let err = import_skill("my-skill", &external, "cursor-user", false, false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("--force or --keep-source"));
        assert!(!repo.join("skills/my-skill").exists());
        assert!(external.join("SKILL.md").is_file());

        std::env::remove_var("CHEF_KITCHEN");
    }

    #[test]
    fn import_fails_clearly_without_kitchen() {
        let _lock = kitchen_env_lock();
        let (tmp, _repo, external) = setup_import_fixture();
        std::env::set_var("CHEF_KITCHEN", tmp.path().join("missing"));

        let err = import_skill("my-skill", &external, "cursor-user", true, false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("no kitchen"), "{err}");
        assert!(external.join("SKILL.md").is_file());

        std::env::remove_var("CHEF_KITCHEN");
    }

    #[test]
    fn import_keep_source_does_not_relink() {
        let _lock = kitchen_env_lock();
        let (_tmp, repo, external) = setup_import_fixture();
        std::env::set_var("CHEF_KITCHEN", &repo);

        let result = import_skill("my-skill", &external, "cursor-user", true, false).unwrap();
        assert!(result.relinked.is_none());
        assert!(repo.join("skills/my-skill/SKILL.md").is_file());
        assert!(external.join("SKILL.md").is_file());
        assert!(!fs::symlink_metadata(&external)
            .unwrap()
            .file_type()
            .is_symlink());

        std::env::remove_var("CHEF_KITCHEN");
    }
}
