# Pi

Pi loads **packages** (`settings.json` `packages`) and **extensions** (`~/.pi/agent/extensions/*.ts`). It does not automatically read this repo.

| Kind | This repo | Pi load path |
|------|-----------|--------------|
| Extension | `extensions/` | `~/.pi/agent/extensions/` |
| Skill | `skills/` | Via package or prompt — see `local-pi-qwen` |
| Rule | `rules/` | Project `AGENTS.md` or Pi package |

## Wire extensions

```bash
kitchen serve
```

Pick **Pi** in the TUI, select `cap-tool-output.ts`, press `l` to link.

Or CLI:

```bash
kitchen serve link pi
```

Symlink: `extensions/cap-tool-output.ts` → `~/.pi/agent/extensions/cap-tool-output.ts`. Run `/reload` in Pi after changes.

## Local Qwen stack

MLX server on `:8080`, compaction, tool caps: [qwen-local.md](qwen-local.md).

**Recipe:** In Kitchen TUI (`kitchen cook`), open **Recipes** → **Local Qwen 9B** → attach to an agent. Bundles skill `local-pi-qwen` + rule `prefer-edit-over-write`.

For Pi sessions in other repos, copy `rules/prefer-edit-over-write.md` into that project's `AGENTS.md` or install via a Pi package.
