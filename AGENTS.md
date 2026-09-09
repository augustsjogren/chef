# slop-chef CLI

Rust TUI/CLI for browsing [slop-kitchen](https://github.com/augustsjogren/slop-kitchen), linking into Cursor/Claude/Pi, and running local models.

**Skills repo:** [augustsjogren/slop-kitchen](https://github.com/augustsjogren/slop-kitchen) at `~/slop-kitchen` — see [SLOP-KITCHEN.md](SLOP-KITCHEN.md).

| Name | What |
|------|------|
| **slop-kitchen** | Skills/rules/extensions repo (`~/slop-kitchen`) |
| **slop-chef** | This CLI tool |

This repo is the **tool** only: source, harness adapters (`harnesses/`), CI.

Prefer `edit` over full-file `write` when changing existing files.

## Layout

| Kind | Path | Rule |
|------|------|------|
| CLI source | `src/` | Rust |
| Harness adapters | `harnesses/` | Install paths in `targets.json` |
| CI fixture | `testdata/pantry/` | Minimal skills for tests only |
| Runtime state | `~/.slop-chef/` | Stove + chat logs (not in git) |

## slop-chef CLI

```bash
./install
slop-chef            # browse ~/slop-kitchen
slop-chef bootstrap
slop-chef serve
slop-chef stove
slop-chef chat
slop-chef prep
```

Set `SLOP_CHEF_PANTRY` to override the default `~/slop-kitchen` path.
