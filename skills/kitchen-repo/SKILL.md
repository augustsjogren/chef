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

Prefer capturing from solved work via **kitchen-add-skill** (install: `kitchen bootstrap`).

For repo canonical skills: write `skills/<name>/SKILL.md`, then `kitchen serve` → link into Cursor or Claude.

## Add a rule

One concern per file, under ~50 lines. Start from `rules/_template.md` if needed. `kitchen serve` → Cursor rules get `.mdc` at dest.

## Pi extension

Put `*.ts` in `extensions/`, then `kitchen serve` → **Pi** harness → link.

## Kitchen CLI

```bash
kitchen            # pantry TUI (all skill dirs on machine)
kitchen bootstrap  # install kitchen + kitchen-add-skill → Cursor user
kitchen serve      # symlink repo pantry
kitchen stove      # local model servers
kitchen chat       # plain local-model chat
kitchen prep       # rebuild after pulling
```

Do not fork the same skill into Cursor/Claude/Pi copies unless a harness cannot share the file.
