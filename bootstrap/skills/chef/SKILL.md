---
name: chef
description: chef CLI and skill workflow — browse, create, edit, import, and link skills and rules on this machine; link the kitchen into Cursor/Claude/Pi. Use when the user mentions chef, kitchen, pantry, serve, or asks to create, edit, install, or manage agent skills and rules.
disable-model-invocation: false
---

# chef

Local CLI for browsing and linking agent skills, rules, and extensions.

## Kitchen

The kitchen is the user's own folder of canonical skills, rules, and extensions. chef reads it from `$CHEF_KITCHEN`, or `~/kitchen` by default.

```text
<kitchen>/
  skills/<name>/SKILL.md   # required folder
  rules/<name>.md          # optional
  extensions/<name>.ts     # optional (Pi)
```

## Quick start

```bash
chef              # pantry TUI — browse all skills/rules on this machine
chef bootstrap    # install chef + chef-add-skill into Cursor (user)
chef serve        # report kitchen links per harness
chef status       # kitchen counts
```

If `chef` is not on PATH, install a release binary or run `./install` from a clone of the chef repo.

## Pantry

`chef` (or `chef pantry`) opens a TUI that aggregates skills and rules from:

- the kitchen
- `~/.cursor/skills` and `~/.cursor/rules`
- `~/.claude/skills` and `~/.agents/skills`
- Project `.cursor/skills`, `.claude/skills`, and `.cursor/rules` when present in cwd

Press `a` to toggle kitchen-only vs. all sources. Press `i` on an external skill to import it into the kitchen.

## Bootstrap

Install chef's bundled meta-skills into your agent:

```bash
chef bootstrap                          # → ~/.cursor/skills
chef bootstrap --harness claude-user    # → ~/.claude/skills
chef bootstrap --harness cursor-project --project .
```

Links `chef` (this skill) and `chef-add-skill` (capture skills from solved problems).

## Serve

`chef serve` prints link status for every harness. Destination paths: `harnesses/targets.json` in the chef repo. Interactive linking lives in the TUI's serve tab.

Non-interactive:

```bash
chef serve status
chef serve link cursor-user
chef serve sync               # link every non-project harness
chef serve --dry-run link pi  # preview only
```

Project harnesses require `--project <path>`.

## Editing existing skills

Edit the canonical skill in the kitchen, not a harness symlink:

```text
<kitchen>/skills/<skill-name>/SKILL.md
```

Harness entries are symlinks, so edits show up immediately. New skills need `chef serve link <harness>` (or `chef serve sync`). Do not maintain separate copies under `~/.cursor/skills`, `~/.claude/skills`, or `~/.pi/agent/skills`.

## Adding skills

Do not hand-write skills from a blank template. After solving a real problem, use the **chef-add-skill** skill to capture the workflow into a new skill file.
