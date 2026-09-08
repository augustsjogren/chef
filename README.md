# kitchen

Personal pantry for agent skills, rules, extensions, and harness adapters — with a cooking-themed TUI.

Canonical definitions live by **kind**. Harness-specific install notes live under `harnesses/`. Do not fork the same skill into Cursor/Claude/Pi copies unless a harness truly cannot share the file.

| Path | What goes here |
|------|----------------|
| `skills/` | Portable Agent Skills (`SKILL.md` directories) |
| `rules/` | Persistent guidance (always-on or glob-scoped) |
| `extensions/` | Runtime hooks (Pi TypeScript, scripts, etc.) |
| `harnesses/` | How to install/symlink a kind into Cursor, Claude, Pi, … |
| `.kitchen/` | Runtime: recipes (committed), agent state + logs (local) |

See [`AGENTS.md`](AGENTS.md) for conventions when editing this repo.

## Install

```bash
./install
```

Installs rustup if needed, adds `~/.cargo/env` to your shell profile, builds `kitchen`, and puts it on `~/.cargo/bin`. Open a **new terminal** (or `source ~/.zshrc`), then:

```bash
kitchen cook                          # TUI
kitchen cook frontend Fix a11y        # spawn/assign + COOKING
kitchen cook frontend -a Fix a11y     # LET HIM COOK
kitchen status
kitchen dismiss frontend            # remove from roster
kitchen serve                       # symlink pantry into Cursor / Claude / Pi
```

After pulling code: `kitchen prep` (rebuild + reinstall). Agent state lives in `.kitchen/` (local).

Set `KITCHEN_ROOT` to point at this repo if you run `kitchen` from elsewhere.

## CLI

| Command | What |
|---------|------|
| `kitchen cook` | Open the TUI |
| `kitchen cook <agent> <task>` | Spawn or assign a **ticket** and start COOKING |
| `kitchen cook <agent> -a <task>` | Same, but LET HIM COOK (autonomous) |
| `kitchen status` | Print roster + pantry counts |
| `kitchen dismiss <agent>` | Remove agent from roster (logs kept) |
| `kitchen prep` | `cargo build` + `cargo install --path .` |
| `kitchen serve` | Harness symlink TUI |
| `kitchen serve link <harness>` | Link all pantry items to one harness |

## TUI

A **ticket** is the task string on an agent — free text like `fix login page`. Required before `[m]` let him cook.

| Status | Meaning |
|--------|---------|
| `RAW` | on roster, no work started |
| `COOKING` | working |
| `LET HIM COOK` | working, autonomous |
| `COOKED` | finished |
| `BURNT` | failed |

Keys: `[enter]` inspect · `[s]` spawn · `[t]` ticket · `[m]` let him cook · `[d]` stop · `[D]` / Delete dismiss · `[c]` plate · `[x]` burnt · `[r]` recipes · `[p]` pantry · `[q]` quit. `j`/`k` move. Pantry: `[a]` attach · `[-]` detach.

Kitchen state is local only — no live Pi/Cursor process yet.

## Pantry (bundled)

| Kind | Name | Purpose |
|------|------|---------|
| Skill | `kitchen-repo` | How to work in this repo |
| Skill | `local-pi-qwen` | Pi + Qwen MLX on `:8080` |
| Rule | `prefer-edit-over-write` | Small local models: edit not write |
| Rule | `kitchen-repo` | Repo conventions (always-on here) |
| Extension | `cap-tool-output.ts` | Pi tool output caps — `kitchen serve link pi` |

Recipe **Local Qwen 9B** (Recipes screen) bundles the Pi skill + edit rule.

Local Pi + Qwen stack notes: [`harnesses/pi/qwen-local.md`](harnesses/pi/qwen-local.md).
