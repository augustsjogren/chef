# Kitchen

The kitchen is your own folder of skills, rules, and extensions. `chef` browses it and links it into your harnesses. It is not part of this repository; keeping it in its own git repository works well.

## Location

`chef` uses the first match:

1. `$CHEF_KITCHEN`
2. `~/kitchen`
3. A folder with `skills/` (or a `kitchen/` subfolder with one) found walking up from the current directory

## Layout

```text
~/kitchen/
  skills/<name>/SKILL.md   # required: one directory per skill
  rules/<name>.md          # optional: Cursor rules
  extensions/<name>.ts     # optional: Pi extensions
```

Only `skills/` is required. An empty one is enough to start:

```bash
mkdir -p ~/kitchen/skills
chef
```

## Filling it

- Write skills directly in `skills/<name>/SKILL.md`.
- Import skills you already have in `~/.cursor/skills`, `~/.claude/skills`, or `~/.agents/skills`: `chef import <name> --keep-source` (or `--force` to replace the original with a symlink), or press `i` in the TUI's all-sources view.
- After solving a real problem, ask your agent to use the bundled `chef-add-skill` skill to capture it.

`testdata/pantry` in this repository is a minimal sample kitchen.
