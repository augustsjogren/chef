use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::config::{bootstrap_skills_dir, harness_by_id};
use crate::item::Item;
use crate::ops::{link_item, ActionResult};

pub const BOOTSTRAP_SKILLS: &[&str] = &["chef", "chef-add-skill"];

/// Embedded so release binaries can bootstrap without a clone of this repository.
const BOOTSTRAP_SKILL_FILES: [(&str, &str); 2] = [
    ("chef", include_str!("../bootstrap/skills/chef/SKILL.md")),
    (
        "chef-add-skill",
        include_str!("../bootstrap/skills/chef-add-skill/SKILL.md"),
    ),
];

/// Write chef's embedded meta-skills under `~/.chef` and symlink them into a
/// harness skill dir (e.g. `cursor-user`).
pub fn bootstrap(harness_id: &str, project: Option<&Path>) -> Result<Vec<(String, ActionResult)>> {
    let harness = harness_by_id(harness_id)?;
    if harness.project && project.is_none() {
        bail!("{harness_id} needs --project");
    }
    if !harness.kinds.contains_key("skills") {
        bail!("{harness_id} has no skills kind");
    }
    let dest_root = harness.dest_dir("skills", project)?;
    let root = bootstrap_skills_dir();
    let mut out = Vec::new();
    for (name, body) in BOOTSTRAP_SKILL_FILES {
        let source = root.join(name);
        fs::create_dir_all(&source).with_context(|| format!("create {}", source.display()))?;
        let skill_md = source.join("SKILL.md");
        fs::write(&skill_md, body).with_context(|| format!("write {}", skill_md.display()))?;
        let item = Item {
            kind: "skills".into(),
            name: name.to_string(),
            source: source.canonicalize().unwrap_or(source),
            dest: dest_root.join(name),
        };
        out.push((name.to_string(), link_item(&item)));
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

    #[test]
    fn embedded_files_match_skill_names() {
        let names: Vec<&str> = BOOTSTRAP_SKILL_FILES.iter().map(|(n, _)| *n).collect();
        assert_eq!(names, BOOTSTRAP_SKILLS);
    }
}
