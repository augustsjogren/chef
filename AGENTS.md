# chef CLI

Rust TUI/CLI for browsing [slop-kitchen](https://github.com/augustsjogren/slop-kitchen), linking into Cursor/Claude/Pi, and running local models.

**Skills repo:** [augustsjogren/slop-kitchen](https://github.com/augustsjogren/slop-kitchen) at `~/slop-kitchen` — see [SLOP-KITCHEN.md](SLOP-KITCHEN.md).

| Name | What |
|------|------|
| **slop-kitchen** | Skills/rules/extensions repo (`~/slop-kitchen`) |
| **chef** | This CLI tool |

This repo is the **tool** only: source, harness adapters (`harnesses/`), CI.

Prefer `edit` over full-file `write` when changing existing files.

## Layout

| Kind | Path | Rule |
|------|------|------|
| CLI source | `src/` | Rust |
| Harness adapters | `harnesses/` | Install paths in `targets.json` |
| CI fixture | `testdata/pantry/` | Minimal skills for tests only |
| Runtime state | `~/.chef/` | Stove + chat logs (not in git) |

## chef CLI

```bash
./install
chef            # browse ~/slop-kitchen
chef bootstrap
chef serve
chef stove
chef chat
chef prep
```

Set `CHEF_PANTRY` to override the default `~/slop-kitchen` path.
