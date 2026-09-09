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
kitchen            # pantry TUI
kitchen pantry     # same
kitchen stove      # burner TUI
kitchen serve      # harness symlink TUI
kitchen chat       # plain local-model chat
kitchen cook       # roster TUI (frozen)
kitchen cook <agent> <task>        # assign ticket, COOKING
kitchen cook <agent> <task> -a     # LET HIM COOK — dispatches to MLX (raw HTTP)
kitchen dismiss <agent>            # remove from roster
kitchen stove simmer mlx           # start MLX Qwen server
kitchen stove cool mlx             # stop MLX server
kitchen prep                       # rebuild after pulling
./dev                              # hack without prep (cargo run → pantry)
kitchen dev -w                     # auto-rebuild on save
```

Dest paths for symlinks: `harnesses/targets.json`.

**Autonomous dispatch:** `-a` / TUI `[m]` sends the ticket to MLX via raw OpenAI HTTP (`/v1/chat/completions`). Kitchen warms the stove if needed. Replies land in `.kitchen/logs/<agent-id>.log`. `[d]` / stop cancels an in-flight dispatch.

Cook TUI: `kitchen cook` (frozen). Rebuild with `kitchen prep`. Agent roster in `.kitchen/state.json`. Pantry is the default TUI (`kitchen`). Harness symlink UI via `kitchen serve`.
