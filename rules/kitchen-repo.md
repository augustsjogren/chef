---
description: Conventions for the kitchen repo — canonical skills/rules/extensions, harness adapters, and Kitchen CLI. Use when editing this repository or adding agent definitions.
alwaysApply: true
globs: ""
---

# Kitchen repo

Canonical definitions live by **kind**. Harness wiring is adapters only.

| Kind | Path | Rule |
|------|------|------|
| Skills | `skills/<name>/SKILL.md` | One skill per directory; copy `_template`, do not edit template in place |
| Rules | `rules/<name>.md` | One concern per file; keep short |
| Extensions | `extensions/` | Pi `*.ts` hooks; symlink via `kitchen serve` |
| Harness notes | `harnesses/<name>/` | Install paths only — no duplicate skill text |
| Runtime state | `.kitchen/` | Local agent state + logs (gitignored) |

When adding a skill or rule: write it once in the kind folder, then add only adapter notes under `harnesses/`.

Do not commit secrets, model weights, or live `settings.json` dumps. Point at paths instead.

Prefer `edit` over full-file `write` when changing existing files.

Install: `./install` then `kitchen cook`. Symlink into Cursor/Claude/Pi: `kitchen serve`.
