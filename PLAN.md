# slop-chef

Same binary. Two jobs. No extra crates.

## Jobs

1. **Pantry** — skills, rules, extensions live in slop-kitchen. `slop-chef` browses them. `slop-chef serve` symlinks them into Cursor / Claude / Pi.
2. **Stove** — local model servers. `slop-chef stove` starts/stops them. `slop-chef chat` talks to whatever is hot.

Linking is only `slop-chef serve`. Pantry does not attach, plate, or dispatch.

## Surfaces

| Command | Job |
|---------|-----|
| `slop-chef` / `slop-chef pantry` | Browse and preview skills, rules, extensions. Default. |
| `slop-chef serve` | Harness symlink UI and CLI. |
| `slop-chef stove` | Burners + model catalog. CLI `simmer` / `cool` stay. |
| `slop-chef chat` | Naked local chat. |

## Layout

Canonical pantry in `~/slop-kitchen`. Tool repo has `harnesses/`. Runtime in `~/.slop-chef/` (stove.json, chats, logs). Dest paths: `harnesses/targets.json`.
