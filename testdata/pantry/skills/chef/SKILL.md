---
name: chef
description: chef CLI — browse skills and rules on this machine, link pantry into Cursor/Claude/Pi, run local models. Use when the user mentions chef, pantry, serve, stove, or wants to view or install agent skills and rules.
disable-model-invocation: true
---

# chef

Local CLI for browsing and linking agent skills, rules, and extensions.

## Quick start

```bash
chef              # pantry TUI — browse all skills/rules on this machine
chef bootstrap    # install chef + chef-add-skill into Cursor (user)
chef serve        # symlink repo pantry into a harness
chef stove        # local model servers (MLX, Ollama)
chef chat         # plain chat with a model on the stove
```

Run `./install` once if `chef` is not on PATH.

## Pantry

`chef` (or `chef pantry`) opens a TUI that aggregates skills and rules from:

- slop-kitchen (`~/slop-kitchen`)
- `~/.cursor/skills` and `~/.cursor/rules`
- `~/.claude/skills`
- Project `.cursor/skills` and `.cursor/rules` when present in cwd

Preview only — use `chef serve` to symlink repo items into a harness.

## Bootstrap

Install chef meta-skills into your agent:

```bash
chef bootstrap                          # → ~/.cursor/skills
chef bootstrap --harness claude-user    # → ~/.claude/skills
chef bootstrap --harness cursor-project --project .
```

Links `chef` (this skill) and `chef-add-skill` (capture skills from solved problems).

## Serve

`chef serve` — TUI to symlink repo skills/rules/extensions into Cursor, Claude, or Pi. Dest paths: `harnesses/targets.json`.

Non-interactive:

```bash
chef serve status
chef serve link cursor-user
```

## Stove

Local models without cloud APIs:

```bash
chef stove simmer mlx    # start MLX Qwen server
chef stove cool mlx      # stop
chef chat                # chat TUI
```

## Adding skills

Do not hand-write skills from a blank template. After solving a real problem, use the **chef-add-skill** skill to capture the workflow into a new skill file.
