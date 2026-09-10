use std::path::Path;

use anyhow::{bail, Result};

use crate::config::{harness_by_id, pantry_root};
use crate::item::Item;
use crate::ops::{link_item, ActionResult};

pub const BOOTSTRAP_SKILLS: &[&str] = &["chef", "chef-add-skill"];

/// Symlink chef meta-skills into a harness skill dir (e.g. `cursor-user`).
pub fn bootstrap(harness_id: &str, project: Option<&Path>) -> Result<Vec<(String, ActionResult)>> {
    let harness = harness_by_id(harness_id)?;
    if harness.project && project.is_none() {
        bail!("{harness_id} needs --project");
    }
    if !harness.kinds.contains_key("skills") {
        bail!("{harness_id} has no skills kind");
    }
    let dest_root = harness.dest_dir("skills", project)?;
    let root = pantry_root();
    let mut out = Vec::new();
    for name in BOOTSTRAP_SKILLS {
        let source = root.join("skills").join(name);
        let skill_md = source.join("SKILL.md");
        if !skill_md.is_file() {
            bail!("missing bootstrap skill: {}", skill_md.display());
        }
        let item = Item {
            kind: "skills".into(),
            name: (*name).to_string(),
            source: source.canonicalize().unwrap_or(source),
            dest: dest_root.join(name),
        };
        out.push(((*name).to_string(), link_item(&item)));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_skill_names() {
        assert_eq!(BOOTSTRAP_SKILLS, &["chef", "chef-add-skill"]);
    }
}
