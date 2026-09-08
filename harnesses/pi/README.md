# Pi

Pi loads **packages** (`settings.json` `packages`) and **extensions** (`~/.pi/agent/extensions/*.ts`). It does not automatically read this repo.

| Kind | This repo | Pi load path |
|------|-----------|--------------|
| Extension | `extensions/` | `~/.pi/agent/extensions/` |
| Skill | `skills/` | Only if you add a package or prompt that points at them |
| Rule | `rules/` | Project `AGENTS.md` or a Pi package |

Live local stack (Qwen MLX on `:8080`, compaction, `cap-tool-output`): [qwen-local.md](qwen-local.md).

When adding a rule for the 9B local model, prefer “use `edit`, avoid full-file `write`” — long writes dominate decode time.
