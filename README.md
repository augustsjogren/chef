# kitchen

CLI for a personal agent skill pantry — browse, import, link into Cursor/Claude/Pi, and run local models.

**Your skills live in [pantry](https://github.com/augustsjogren/pantry)** (`~/kitchen`), not in this repo. See [PANTRY.md](PANTRY.md).

| Repo | Role |
|------|------|
| [augustsjogren/kitchen](https://github.com/augustsjogren/kitchen) | This tool (Rust CLI, harness adapters) |
| [augustsjogren/pantry](https://github.com/augustsjogren/pantry) | Your skills, rules, extensions → `~/kitchen` |

CI runs `cargo fmt --check`, `cargo clippy`, and `cargo test` on push/PR.

## Install

```bash
./install
```

Then clone [pantry](https://github.com/augustsjogren/pantry) to `~/kitchen`, or set `KITCHEN_PANTRY`.

```bash
kitchen bootstrap    # meta-skills → Cursor
kitchen serve        # link ~/kitchen → harnesses
kitchen              # pantry TUI
```

After pulling **this** repo: `kitchen prep`.

## CLI

| Command | What |
|---------|------|
| `kitchen` / `kitchen pantry` | Browse `~/kitchen` (+ all sources with **A**) |
| `kitchen import <name>` | Copy external skill into pantry |
| `kitchen bootstrap` | Install kitchen meta-skills into a harness |
| `kitchen serve` | Symlink pantry into Cursor / Claude / Pi |
| `kitchen stove` / `kitchen chat` | Local models |
| `kitchen status` | Pantry counts |
| `kitchen prep` | Rebuild + reinstall CLI |

Runtime state (stove, chat logs): `~/.kitchen/` — not synced.

## Env

| Variable | Default | Purpose |
|----------|---------|---------|
| `KITCHEN_PANTRY` | `~/kitchen` | Skills, rules, extensions |
| `KITCHEN_ROOT` | — | Legacy alias for `KITCHEN_PANTRY` |

Harness paths: `harnesses/targets.json` in this repo.
