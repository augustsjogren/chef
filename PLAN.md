# chef

One binary. One job. No extra crates.

## Job

**Pantry**: skills, rules, and extensions live in kitchen. `chef` browses them. `chef serve` symlinks them into Cursor / Claude / Pi.

Linking is only `chef serve`. Pantry does not attach, plate, or dispatch.

## Surfaces

| Command | Job |
|---------|-----|
| `chef` / `chef pantry` | Browse and preview skills, rules, extensions. Default. |
| `chef serve` | Harness symlink UI and CLI. |

## Layout

Canonical pantry in `~/kitchen`. Tool repo has `harnesses/`. Dest paths: `harnesses/targets.json`.
