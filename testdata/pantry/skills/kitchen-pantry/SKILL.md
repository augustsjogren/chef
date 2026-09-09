---
name: kitchen-pantry
description: Work with the personal kitchen pantry in ~/kitchen (dotfiles) — skills, rules, extensions. Use when adding or editing agent definitions, running kitchen serve, or managing the dotfiles pantry.
disable-model-invocation: true
---

# Kitchen pantry (dotfiles)

Canonical definitions live in **`~/kitchen`** — synced via [dotfiles](https://github.com/augustsjogren/dotfiles).

The [kitchen CLI](https://github.com/augustsjogren/kitchen) is the tool; this folder is the content.

## Layout

- `skills/` — portable `SKILL.md` directories
- `rules/` — short persistent guidance (`.md`)
- `extensions/` — Pi TypeScript hooks
- Harness wiring — kitchen repo `harnesses/targets.json` (not duplicated here)

## Add a skill

Prefer **kitchen-add-skill** after solving a real problem (`kitchen bootstrap` to install).

Or write `skills/<name>/SKILL.md` directly, then `kitchen serve`.

## Add a rule

One concern per file. Start from `rules/_template.md` if needed.

## Kitchen CLI

```bash
kitchen            # browse ~/kitchen
kitchen bootstrap  # meta-skills → Cursor
kitchen serve      # link pantry → harnesses
kitchen import …   # promote external skill into ~/kitchen
```

Do not fork the same skill into Cursor/Claude/Pi copies — `kitchen serve` symlinks from here.
