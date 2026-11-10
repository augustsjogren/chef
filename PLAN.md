# chef

Same binary. Two jobs. No extra crates.

## Jobs

1. **Pantry** — skills, rules, extensions live in kitchen. `chef` browses them. `chef serve` symlinks them into Cursor / Claude / Pi.
2. **Stove** — local model servers. `chef stove` starts/stops them.

Linking is only `chef serve`. Pantry does not attach, plate, or dispatch.

## Surfaces

| Command | Job |
|---------|-----|
| `chef` / `chef pantry` | Browse and preview skills, rules, extensions. Default. |
| `chef serve` | Harness symlink UI and CLI. |
| `chef stove` | Burners + model catalog. CLI `simmer` / `cool` stay. |

## Layout

Canonical pantry in `~/kitchen`. Tool repo has `harnesses/`. Runtime in `~/.chef/` (stove.json, logs). Dest paths: `harnesses/targets.json`.
