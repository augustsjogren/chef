# slop-chef

CLI for browsing [slop-kitchen](https://github.com/augustsjogren/slop-kitchen), importing skills, linking into Cursor/Claude/Pi, and running local models.

**Skills live in [slop-kitchen](https://github.com/augustsjogren/slop-kitchen)** (`~/slop-kitchen`), not here. See [SLOP-KITCHEN.md](SLOP-KITCHEN.md).

| Repo | Role |
|------|------|
| [augustsjogren/slop-kitchen](https://github.com/augustsjogren/slop-kitchen) | Skills, rules, extensions |
| [augustsjogren/slop-chef](https://github.com/augustsjogren/slop-chef) | This CLI |

## Install

```bash
./install
git clone git@github.com:augustsjogren/slop-kitchen.git ~/slop-kitchen
slop-chef bootstrap && slop-chef serve
slop-chef
```

After pulling **this** repo: `slop-chef prep`.

## CLI

The binary is **`slop-chef`**. Bare `slop-chef` opens the browse TUI. Skills live in the **slop-kitchen** repo.

| Command | What |
|---------|------|
| `slop-chef` | Browse slop-kitchen (**A** = all sources, **I** = import) |
| `slop-chef import <name>` | Copy external skill into slop-kitchen |
| `slop-chef bootstrap` | Install meta-skills into a harness |
| `slop-chef serve` | Symlink slop-kitchen into Cursor / Claude / Pi |
| `slop-chef stove` / `slop-chef chat` | Local models |
| `slop-chef status` | slop-kitchen counts |
| `slop-chef prep` | Rebuild + reinstall CLI |

Runtime state: `~/.slop-chef/`

## Env

| Variable | Default |
|----------|---------|
| `SLOP_CHEF_PANTRY` | `~/slop-kitchen` |
