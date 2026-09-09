# kitchen

CLI for browsing [slop-kitchen](https://github.com/augustsjogren/slop-kitchen), importing skills, linking into Cursor/Claude/Pi, and running local models.

**Skills live in [slop-kitchen](https://github.com/augustsjogren/slop-kitchen)** (`~/slop-kitchen`), not here. See [SLOP-KITCHEN.md](SLOP-KITCHEN.md).

| Repo | Role |
|------|------|
| [augustsjogren/slop-kitchen](https://github.com/augustsjogren/slop-kitchen) | Skills, rules, extensions |
| [augustsjogren/kitchen](https://github.com/augustsjogren/kitchen) | This CLI |

## Install

```bash
./install
git clone git@github.com:augustsjogren/slop-kitchen.git ~/slop-kitchen
kitchen bootstrap && kitchen serve
kitchen
```

After pulling **this** repo: `kitchen prep`.

## CLI

| Command | What |
|---------|------|
| `kitchen` | Browse `~/slop-kitchen` (**A** = all sources, **I** = import) |
| `kitchen import <name>` | Copy external skill into slop-kitchen |
| `kitchen bootstrap` | Install meta-skills into a harness |
| `kitchen serve` | Symlink slop-kitchen into Cursor / Claude / Pi |
| `kitchen stove` / `kitchen chat` | Local models |
| `kitchen status` | Skill counts |
| `kitchen prep` | Rebuild + reinstall CLI |

Runtime state: `~/.kitchen/`

## Env

| Variable | Default |
|----------|---------|
| `KITCHEN_PANTRY` | `~/slop-kitchen` |
| `KITCHEN_ROOT` | legacy alias |
