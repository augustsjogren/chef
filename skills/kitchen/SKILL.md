---
name: kitchen
description: Kitchen CLI — browse skills and rules on this machine, link pantry into Cursor/Claude/Pi, run local models. Use when the user mentions kitchen, pantry, serve, stove, or wants to view or install agent skills and rules.
disable-model-invocation: true
---

# Kitchen CLI

Kitchen is a local CLI for browsing and linking agent skills, rules, and extensions.

## Quick start

```bash
kitchen              # pantry TUI — browse all skills/rules on this machine
kitchen bootstrap    # install kitchen + kitchen-add-skill into Cursor (user)
kitchen serve        # symlink repo pantry into a harness
kitchen stove        # local model servers (MLX, Ollama)
kitchen chat         # plain chat with a model on the stove
```

Run `./install` once if `kitchen` is not on PATH.

## Pantry

`kitchen` (or `kitchen pantry`) opens a TUI that aggregates skills and rules from:

- This repo's `skills/`, `rules/`, `extensions/` (when run from a kitchen checkout)
- `~/.cursor/skills` and `~/.cursor/rules`
- `~/.claude/skills`
- Project `.cursor/skills` and `.cursor/rules` when present in cwd

Preview only — use `kitchen serve` to symlink repo items into a harness.

## Bootstrap

Install kitchen's meta-skills into your agent:

```bash
kitchen bootstrap                          # → ~/.cursor/skills
kitchen bootstrap --harness claude-user    # → ~/.claude/skills
kitchen bootstrap --harness cursor-project --project .
```

Links `kitchen` (this skill) and `kitchen-add-skill` (capture skills from solved problems).

## Serve

`kitchen serve` — TUI to symlink repo skills/rules/extensions into Cursor, Claude, or Pi. Dest paths: `harnesses/targets.json`.

Non-interactive:

```bash
kitchen serve status
kitchen serve link cursor-user
```

## Stove

Local models without cloud APIs:

```bash
kitchen stove simmer mlx    # start MLX Qwen server
kitchen stove cool mlx      # stop
kitchen chat                # chat TUI
```

## Adding skills

Do not hand-write skills from a blank template. After solving a real problem, use the **kitchen-add-skill** skill to capture the workflow into a new skill file.
