# Kitchen CLI

Rust TUI/CLI for browsing a personal skill pantry, linking into Cursor/Claude/Pi, and running local models.

**Pantry contents** (skills, rules, extensions) live in [augustsjogren/pantry](https://github.com/augustsjogren/pantry) at `~/kitchen` — see [PANTRY.md](PANTRY.md).

This repo is the **tool** only: source, harness adapters (`harnesses/`), CI.

Prefer `edit` over full-file `write` when changing existing files.

## Layout

| Kind | Path | Rule |
|------|------|------|
| CLI source | `src/` | Rust |
| Harness adapters | `harnesses/` | Install paths in `targets.json` — no duplicate skill text |
| CI fixture | `testdata/pantry/` | Minimal skills for tests only |
| Runtime state | `~/.kitchen/` | Stove + chat logs (not in git) |

## Kitchen CLI

```bash
./install          # once: rust + kitchen on PATH
kitchen            # pantry TUI (reads ~/kitchen)
kitchen bootstrap  # install meta-skills into Cursor user dir
kitchen serve      # symlink pantry into Cursor / Claude / Pi
kitchen stove      # local model servers
kitchen chat       # plain local-model chat
kitchen prep       # rebuild after pulling this repo
./dev              # hack without prep
```

Set `KITCHEN_PANTRY` to override the default `~/kitchen` path.
