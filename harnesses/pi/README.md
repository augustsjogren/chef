# Pi

Pi loads skills from `~/.pi/agent/skills/`, plus packages and extensions. `chef serve` can link skills and extensions into Pi.

| Kind | This repo | Pi load path |
|------|-----------|--------------|
| Extension | `extensions/` | `~/.pi/agent/extensions/` |
| Skill | `skills/` | `~/.pi/agent/skills/` |
| Rule | `rules/` | Project `AGENTS.md` or Pi package |

## Wire skills and extensions

```bash
chef serve
```

Pick **Pi** in the TUI, select skills or extensions, press `l` to link.

Or CLI:

```bash
chef serve link pi
```

Skills are symlinked into `~/.pi/agent/skills/`; extensions are symlinked into `~/.pi/agent/extensions/`. Restart Pi after skill changes; run `/reload` after extension changes.

## Local Qwen stack

MLX server on `:8080`, compaction, tool caps: [qwen-local.md](qwen-local.md).

For Pi sessions in other repos, copy `rules/prefer-edit-over-write.md` into that project's `AGENTS.md` or install via a Pi package.
