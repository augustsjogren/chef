use std::fs;
use std::path::Path;

use crate::item::{classify, Item, LinkState};

#[derive(Debug)]
pub enum ActionResult {
    Linked,
    AlreadyLinked,
    Unlinked,
    AlreadyMissing,
    Blocked(String),
}

impl std::fmt::Display for ActionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Linked => write!(f, "linked"),
            Self::AlreadyLinked => write!(f, "already linked"),
            Self::Unlinked => write!(f, "unlinked"),
            Self::AlreadyMissing => write!(f, "already missing"),
            Self::Blocked(msg) => write!(f, "{msg}"),
        }
    }
}

pub fn link_item(item: &Item) -> ActionResult {
    match classify(item) {
        LinkState::Linked => ActionResult::AlreadyLinked,
        LinkState::Blocked => ActionResult::Blocked(format!(
            "blocked (real path exists): {}",
            item.dest.display()
        )),
        LinkState::Foreign => ActionResult::Blocked(format!(
            "blocked (symlink elsewhere): {}",
            item.dest.display()
        )),
        LinkState::Missing => {
            if let Some(parent) = item.dest.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    return ActionResult::Blocked(format!("mkdir {}: {e}", parent.display()));
                }
            }
            match symlink(&item.source, &item.dest) {
                Ok(()) => ActionResult::Linked,
                Err(e) => ActionResult::Blocked(format!("symlink failed: {e}")),
            }
        }
    }
}

pub fn unlink_item(item: &Item) -> ActionResult {
    match classify(item) {
        LinkState::Missing => ActionResult::AlreadyMissing,
        // Linked and Foreign are both symlinks; removing one never touches
        // the target. Foreign removal is how a mis-pointed link is healed:
        // unlink the old link, then link fresh from the kitchen.
        LinkState::Linked | LinkState::Foreign => match fs::remove_file(&item.dest) {
            Ok(()) => ActionResult::Unlinked,
            Err(e) => ActionResult::Blocked(format!("unlink failed: {e}")),
        },
        other => ActionResult::Blocked(format!("blocked ({}): {}", other, item.dest.display())),
    }
}

#[cfg(unix)]
fn symlink(source: &Path, dest: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, dest)
}

#[cfg(not(unix))]
fn symlink(_source: &Path, _dest: &Path) -> std::io::Result<()> {
    Err(std::io::Error::other("symlinks only implemented on unix"))
}
