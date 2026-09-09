# Kitchen split

Same binary. Four commands. Pantry stays the source of truth — no extra crates.

## Surfaces

| Command | Job |
|---------|-----|
| `kitchen` / `kitchen pantry` | Browse and preview skills, rules, extensions, recipes. Default. |
| `kitchen stove` | Burners + model catalog. CLI `simmer` / `cool` stay. |
| `kitchen serve` | Harness symlink UI. Keep the command; use the same theme as chat. |
| `kitchen chat` | Naked local chat. Done. |

Bare `kitchen` opens pantry, not the roster. Recipes are named plates of pantry items. “Attach” means serve into a harness, not hang on a ghost cook.

## Sequence

1. **Pantry as front door.** New `src/pantry_ui.rs`. Preview file bodies. No selected-agent requirement. Wire `kitchen` and `kitchen pantry` here.
2. **Stove TUI.** Lift the stove screen out of `src/ui.rs` into `src/stove_ui.rs`. `kitchen stove` with no args opens it. Chat already consumes the catalog.
3. **Serve chrome.** Point `src/tui.rs` at `theme.rs` (rename to `serve_ui.rs` when convenient). No product change.
4. **Starve cook.** Freeze the roster behind `kitchen cook` only. Stop adding keys. `[m]` / inspect-log / `src/runtime.rs` are leftover — delete with the TUI when pantry + stove feel like the product. CLI `cook -a` may die in the same pass.
5. **Line, later.** New command when Pi / Claude / OpenCode are real backends. Status only, no transcript. Not a refactor of cook.

## Leave alone

Chat. Stove process control. Canonical `skills/` `rules/` `extensions/` `harnesses/`. `.kitchen/chats/`.

## Done when

`kitchen` shows the pantry. Stove and serve are their own screens. Cook is gone or a hidden stub. Line is not started.

Steps 1–4 shipped. Cook remains behind `kitchen cook`. Line is not started.
