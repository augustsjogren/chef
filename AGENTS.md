# This repo

**kitchen** — canonical definitions: `skills/`, `rules/`, `extensions/`.

When adding a skill or rule, write it once in the matching kind folder. Then add only the harness adapter notes or install steps under `harnesses/<name>/`.

Do not put live secrets, model weights, or `~/.pi/agent/settings.json` dumps here. Point at paths instead.

Prefer `edit` over full-file `write` when changing existing files.

## Layout

| Kind | Path | Rule |
|------|------|------|
| Skills | `skills/<name>/SKILL.md` | One skill per directory; copy `_template`, do not edit template in place |
| Rules | `rules/<name>.md` | One concern per file; keep short |
| Extensions | `extensions/` | Pi `*.ts` hooks; symlink via `kitchen serve` |
| Harness notes | `harnesses/<name>/` | Install paths only — no duplicate skill text |
| Runtime state | `.kitchen/` | Local agent state + logs (gitignored except `recipes.json`) |

## Kitchen CLI

```bash
./install          # once: rust + kitchen on PATH
kitchen cook       # TUI
kitchen cook <agent> <task>   # assign ticket (local state until runtime wired)
kitchen dismiss <agent>       # remove from roster
kitchen serve      # symlink pantry into Cursor / Claude / Pi
kitchen prep       # rebuild after pulling
```

Dest paths for symlinks: `harnesses/targets.json`.

Cook TUI: `./install` once, then `kitchen cook`. Rebuild with `kitchen prep`. Kitchen state is local (`.kitchen/state.json`, `.kitchen/logs/`) — not a live agent runtime yet. Assign work with `kitchen cook <agent> <task>`. Harness symlink UI: `kitchen serve`.
