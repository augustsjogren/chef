# skills

One directory per skill. Required file: `SKILL.md` with YAML frontmatter (`name`, `description`).

```
skills/<skill-name>/
  SKILL.md
  reference.md      # optional, read on demand
  scripts/          # optional
```

`name`: lowercase, hyphens, max 64 chars. `description`: third person, what + when.

Start from `_template/`. Copy the folder; do not edit the template in place for a real skill.

Typical install targets (see `harnesses/`):

- Cursor project: `.cursor/skills/<name>/`
- Cursor user: `~/.cursor/skills/<name>/`
- Claude: `~/.claude/skills/<name>/` or project `.claude/skills/<name>/`
