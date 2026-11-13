# chef CLI

Rust TUI/CLI for browsing a kitchen (any folder with `skills/`) and linking it into Cursor/Claude/Pi.

**Kitchen:** the user's own skills folder at `~/kitchen` (or `CHEF_KITCHEN`) — see [KITCHEN.md](KITCHEN.md).

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
| Bundled skills | `bootstrap/skills/` | `chef` + `chef-add-skill`, linked by `chef bootstrap` |
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
