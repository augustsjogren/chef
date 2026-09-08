# agents

Personal source of truth for agent skills, rules, harness adapters, and extensions.

Definitions live by **kind**. Harness-specific wiring lives under `harnesses/`. Do not fork the same skill into Cursor/Claude/Pi copies unless a harness truly cannot share the file.

| Path | What goes here |
|------|----------------|
| `skills/` | Portable Agent Skills (`SKILL.md` directories) |
| `rules/` | Persistent guidance (always-on or glob-scoped) |
| `extensions/` | Runtime hooks (Pi TypeScript, scripts, etc.) |
| `harnesses/` | How to install/symlink a kind into Cursor, Claude, Pi, … |
| `.kitchen/` | Runtime: recipes (committed), agent state + logs (local) |

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
kitchen serve                         # symlink pantry into Cursor / Claude / Pi
```

After pulling code: `kitchen prep` (rebuild + reinstall). Data lives in `.kitchen/`.

| Status | Meaning |
|--------|---------|
| `RAW` | not started |
| `COOKING` | working |
| `LET HIM COOK` | working, autonomous |
| `COOKED` | finished |
| `BURNT` | failed |

Keys (kitchen): `[enter]` inspect · `[s]` spawn · `[t]` ticket · `[m]` let him cook · `[d]` stop · `[D]` / Delete dismiss · `[c]` plate · `[x]` burnt · `[q]` quit. `j`/`k` move.

## Pantry (included)

| Kind | Name | Purpose |
|------|------|---------|
| Skill | `agents-repo` | How to work in this repo |
| Skill | `local-pi-qwen` | Pi + Qwen MLX on `:8080` |
| Rule | `prefer-edit-over-write` | Small local models: edit not write |
| Rule | `agents-repo` | Repo conventions (always-on here) |
| Extension | `cap-tool-output.ts` | Pi tool output caps — `kitchen serve link pi` |

Recipe **Local Qwen 9B** bundles the Pi skill + edit rule.

Existing local Pi + Qwen notes: [`harnesses/pi/qwen-local.md`](harnesses/pi/qwen-local.md).
