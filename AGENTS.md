# chef CLI

Rust TUI/CLI for browsing [kitchen](https://github.com/augustsjogren/kitchen), and linking into Cursor/Claude/Pi.

**Skills repo:** [augustsjogren/kitchen](https://github.com/augustsjogren/kitchen) at `~/kitchen` — see [KITCHEN.md](KITCHEN.md).

| Name | What |
|------|------|
| **kitchen** | Skills/rules/extensions repo (`~/kitchen`) |
| **chef** | This CLI tool |

This repo is the **tool** only: source, harness adapters (`harnesses/`), CI.

Prefer `edit` over full-file `write` when changing existing files.

## Layout

| Kind | Path | Rule |
|------|------|------|
| CLI source | `src/` | Rust |
| Harness adapters | `harnesses/` | Install paths in `targets.json` |
| CI fixture | `testdata/pantry/` | Minimal skills for tests only |

## chef CLI

```bash
./install
chef            # browse ~/kitchen
chef bootstrap
chef serve
chef prep
```

Set `CHEF_KITCHEN` to override the default `~/kitchen` path.
