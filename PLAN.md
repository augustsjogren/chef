# Kitchen

Same binary. Two jobs. No extra crates.

## Jobs

1. **Shelf** — skills, rules, extensions live in this repo. `kitchen` browses them. `kitchen serve` symlinks them into Cursor / Claude / Pi.
2. **Stove** — local model servers. `kitchen stove` starts/stops them. `kitchen chat` talks to whatever is hot.

Linking is only `kitchen serve`. Pantry does not attach, plate, or dispatch.

## Surfaces

| Command | Job |
|---------|-----|
| `kitchen` / `kitchen pantry` | Browse and preview skills, rules, extensions. Default. |
| `kitchen serve` | Harness symlink UI and CLI. |
| `kitchen stove` | Burners + model catalog. CLI `simmer` / `cool` stay. |
| `kitchen chat` | Naked local chat. |

## Layout

Canonical `skills/` `rules/` `extensions/` `harnesses/`. Runtime in `.kitchen/` (stove.json, chats, logs). Dest paths: `harnesses/targets.json`.
