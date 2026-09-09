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
- `.kitchen/` — local stove + chat logs

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
kitchen            # pantry TUI
kitchen serve      # symlink pantry
kitchen stove      # local model servers
kitchen chat       # plain local-model chat
kitchen prep       # rebuild after pulling
```

Do not fork the same skill into Cursor/Claude/Pi copies unless a harness cannot share the file.
