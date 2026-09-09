# Kitchen split

Same binary. Two jobs. No extra crates.

## Jobs

1. **Shelf** — skills, rules, extensions live in this repo. `kitchen serve` symlinks them into Cursor / Claude / Pi.
2. **Stove** — local model servers. `kitchen stove` starts/stops them. `kitchen chat` talks to whatever is hot.

`kitchen` opens the pantry (look, don’t link). Linking is only `kitchen serve`.

## Surfaces

| Command | Job |
|---------|-----|
| `kitchen` / `kitchen pantry` | Browse and preview skills, rules, extensions. Default. |
| `kitchen serve` | Harness symlink UI and CLI. |
| `kitchen stove` | Burners + model catalog. CLI `simmer` / `cool` stay. |
| `kitchen chat` | Naked local chat. |

## Leave alone

Canonical `skills/` `rules/` `extensions/` `harnesses/`. Stove process control. `.kitchen/chats/`.

## Done when

Cook roster, recipes, and Line are gone. Pantry is a shelf. Serve is the linker. Stove and chat are the local-model side.

Cook and recipes are gone. Line is not started.
