# This repo

Canonical definitions: `skills/`, `rules/`, `extensions/`.

When adding a skill or rule, write it once in the matching kind folder. Then add only the harness adapter notes or install steps under `harnesses/<name>/`.

Do not put live secrets, model weights, or `~/.pi/agent/settings.json` dumps here. Point at paths instead.

Prefer `edit` over full-file `write` when changing existing files.

Cook TUI: `./install` once, then `kitchen cook`. Rebuild with `kitchen prep`. Kitchen state is local (`.kitchen/state.json`, `.kitchen/logs/`) — not a live agent runtime yet. Assign work with `kitchen cook <agent> <task>`. Harness symlink UI: `kitchen serve`. Dest paths in `harnesses/targets.json`.
