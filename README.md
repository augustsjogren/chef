# chef

CLI for browsing [kitchen](https://github.com/augustsjogren/kitchen), importing skills, linking into Cursor/Claude/Pi, and running local models.

**Skills live in [kitchen](https://github.com/augustsjogren/kitchen)** (`~/kitchen`), not here. See [KITCHEN.md](KITCHEN.md).

| Repo | Role |
|------|------|
| [augustsjogren/kitchen](https://github.com/augustsjogren/kitchen) | Skills, rules, extensions |
| [augustsjogren/chef](https://github.com/augustsjogren/chef) | This CLI |

## Install

```bash
./install
git clone git@github.com:augustsjogren/kitchen.git ~/kitchen
chef bootstrap && chef serve
chef
```

After pulling **this** repo: `chef prep`.

## CLI

The binary is **`chef`**. Bare `chef` opens the browse TUI. Skills live in the **kitchen** repo.

| Command | What |
|---------|------|
| `chef` | Browse kitchen (**A** = all sources, **I** = import) |
| `chef import <name>` | Copy external skill into kitchen |
| `chef bootstrap` | Install meta-skills into a harness |
| `chef serve` | Symlink kitchen into Cursor / Claude / Pi |
| `chef stove` / `chef chat` | Local models |
| `chef status` | kitchen counts |
| `chef prep` | Rebuild + reinstall CLI |

Runtime state: `~/.chef/`

## Env

| Variable | Default |
|----------|---------|
| `CHEF_KITCHEN` | `~/kitchen` |
