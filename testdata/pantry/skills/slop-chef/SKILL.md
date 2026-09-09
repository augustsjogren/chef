---
name: slop-chef
description: slop-chef CLI — browse skills and rules on this machine, link pantry into Cursor/Claude/Pi, run local models. Use when the user mentions slop-chef, pantry, serve, stove, or wants to view or install agent skills and rules.
disable-model-invocation: true
---

# slop-chef

Local CLI for browsing and linking agent skills, rules, and extensions.

## Quick start

```bash
slop-chef              # pantry TUI — browse all skills/rules on this machine
slop-chef bootstrap    # install slop-chef + slop-chef-add-skill into Cursor (user)
slop-chef serve        # symlink repo pantry into a harness
slop-chef stove        # local model servers (MLX, Ollama)
slop-chef chat         # plain chat with a model on the stove
```

Run `./install` once if `slop-chef` is not on PATH.

## Pantry

`slop-chef` (or `slop-chef pantry`) opens a TUI that aggregates skills and rules from:

- slop-kitchen (`~/slop-kitchen`)
- `~/.cursor/skills` and `~/.cursor/rules`
- `~/.claude/skills`
- Project `.cursor/skills` and `.cursor/rules` when present in cwd

Preview only — use `slop-chef serve` to symlink repo items into a harness.

## Bootstrap

Install slop-chef meta-skills into your agent:

```bash
slop-chef bootstrap                          # → ~/.cursor/skills
slop-chef bootstrap --harness claude-user    # → ~/.claude/skills
slop-chef bootstrap --harness cursor-project --project .
```

Links `slop-chef` (this skill) and `slop-chef-add-skill` (capture skills from solved problems).

## Serve

`slop-chef serve` — TUI to symlink repo skills/rules/extensions into Cursor, Claude, or Pi. Dest paths: `harnesses/targets.json`.

Non-interactive:

```bash
slop-chef serve status
slop-chef serve link cursor-user
```

## Stove

Local models without cloud APIs:

```bash
slop-chef stove simmer mlx    # start MLX Qwen server
slop-chef stove cool mlx      # stop
slop-chef chat                # chat TUI
```

## Adding skills

Do not hand-write skills from a blank template. After solving a real problem, use the **slop-chef-add-skill** skill to capture the workflow into a new skill file.
