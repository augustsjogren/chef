---
name: kitchen-repo
description: Work in the kitchen repo — skills, rules, extensions, harnesses, and Kitchen CLI. Use when adding or editing agent definitions, running kitchen serve, or scaffolding skills and rules in this repository.
disable-model-invocation: true
---

# Kitchen repo workflow

## Layout

- `skills/` — portable `SKILL.md` directories
- `rules/` — short persistent guidance (`.md`)
- `extensions/` — Pi TypeScript hooks
- `harnesses/` — where each harness loads definitions (`targets.json` for symlink paths)
- `.kitchen/` — local Kitchen runtime (agents, recipes, logs)

## Add a skill

1. Copy `skills/_template/` → `skills/<name>/`
2. Edit `SKILL.md` frontmatter (`name`, `description`) and body
3. `kitchen serve` → link into Cursor (`cursor-user` or `cursor-project`) or Claude

## Add a rule

1. Copy `rules/_template.md` → `rules/<concern>.md`
2. One concern, under ~50 lines
3. `kitchen serve` → Cursor rules get `.mdc` extension at dest

## Pi extension

Put `*.ts` in `extensions/`, then `kitchen serve` → **Pi** harness → link.

## Kitchen CLI

```bash
kitchen cook                    # TUI
kitchen cook <agent> <task>     # assign work (local state until runtime wired)
kitchen dismiss <agent>         # remove from roster
kitchen serve                   # symlink pantry
kitchen prep                    # rebuild after pulling
```

## Recipes

Reusable skill+rule plates live in `.kitchen/recipes.json`. Example: **Local Qwen 9B** bundles `local-pi-qwen` + `prefer-edit-over-write`.

Do not fork the same skill into Cursor/Claude/Pi copies unless a harness cannot share the file.
