# chef

CLI for browsing [slop-kitchen](https://github.com/augustsjogren/slop-kitchen), importing skills, linking into Cursor/Claude/Pi, and running local models.

**Skills live in [slop-kitchen](https://github.com/augustsjogren/slop-kitchen)** (`~/slop-kitchen`), not here. See [SLOP-KITCHEN.md](SLOP-KITCHEN.md).

| Repo | Role |
|------|------|
| [augustsjogren/slop-kitchen](https://github.com/augustsjogren/slop-kitchen) | Skills, rules, extensions |
| [augustsjogren/chef](https://github.com/augustsjogren/chef) | This CLI |

## Install

```bash
./install
git clone git@github.com:augustsjogren/slop-kitchen.git ~/slop-kitchen
chef bootstrap && chef serve
chef
```

After pulling **this** repo: `chef prep`.

## CLI

The binary is **`chef`**. Bare `chef` opens the browse TUI. Skills live in the **slop-kitchen** repo.

| Command | What |
|---------|------|
| `chef` | Browse slop-kitchen (**A** = all sources, **I** = import) |
| `chef import <name>` | Copy external skill into slop-kitchen |
| `chef bootstrap` | Install meta-skills into a harness |
| `chef serve` | Symlink slop-kitchen into Cursor / Claude / Pi |
| `chef stove` / `chef chat` | Local models |
| `chef status` | slop-kitchen counts |
| `chef prep` | Rebuild + reinstall CLI |

Runtime state: `~/.chef/`

## Env

| Variable | Default |
|----------|---------|
| `CHEF_PANTRY` | `~/slop-kitchen` |
