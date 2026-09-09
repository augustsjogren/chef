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
| Runtime state | `.kitchen/` | Local stove + chat logs (gitignored) |

## Kitchen CLI

```bash
./install          # once: rust + kitchen on PATH
kitchen            # pantry TUI
kitchen pantry     # same
kitchen serve      # harness symlink TUI
kitchen stove      # burner TUI
kitchen chat       # plain local-model chat
kitchen stove simmer mlx           # start MLX Qwen server
kitchen stove cool mlx             # stop MLX server
kitchen prep                       # rebuild after pulling
./dev                              # hack without prep (cargo run → pantry)
kitchen dev -w                     # auto-rebuild on save
```

Dest paths for symlinks: `harnesses/targets.json`.

Pantry is the default TUI (`kitchen`). Link into Cursor / Claude / Pi with `kitchen serve`. Local models: `kitchen stove` and `kitchen chat`.
