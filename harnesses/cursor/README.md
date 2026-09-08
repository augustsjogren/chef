# Cursor

| Kind | This repo | Cursor load path |
|------|-----------|------------------|
| Skill | `skills/<name>/` | Project `.cursor/skills/<name>/` or user `~/.cursor/skills/<name>/` |
| Rule | `rules/<name>.md` | `.cursor/rules/<name>.mdc` (same body; Cursor frontmatter: `description`, `globs`, `alwaysApply`) |
| Skill (built-in) | — | Never write to `~/.cursor/skills-cursor/` |

Prefer project copies when the repo should share the skill; user dir when it is personal across all workspaces.
