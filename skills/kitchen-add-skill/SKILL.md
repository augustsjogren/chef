---
name: kitchen-add-skill
description: Capture a solved problem into a new agent skill. Use when the user asks to save a workflow as a skill, add a skill from this conversation, or turn recent work into reusable guidance.
disable-model-invocation: true
---

# Add a skill from solved work

Skills born from real problems beat blank templates. Capture after the user has actually solved something — not at the start of a task.

## When to use

- User finished a hard problem and says "save this as a skill"
- A workflow should repeat across projects
- Domain knowledge emerged in chat that the agent cannot infer

## Before writing

Confirm with the user:

1. **Name** — kebab-case, max 64 chars (e.g. `deploy-staging`, `review-sql-migrations`)
2. **Scope** — personal (`~/.cursor/skills/` or `~/.claude/skills/`) vs project (`.cursor/skills/` in repo)
3. **Trigger** — exact phrases or situations when the agent should load this skill

If the user gave verbatim wording for the skill body, use it **verbatim**.

## What to extract from the conversation

| Section | Source |
|---------|--------|
| `description` (frontmatter) | Third person: what it does + when to use it |
| Trigger scenarios | User's original ask and follow-ups |
| Steps | What actually worked, in order |
| Pitfalls | Errors hit and fixes applied |
| Examples | Concrete input/output from this session |

Do not invent steps the conversation never tried.

## File layout

```
<skill-dir>/
  SKILL.md          # required
  reference.md      # optional — long detail, read on demand
  scripts/          # optional
```

### SKILL.md template

```markdown
---
name: skill-name
description: Does X. Use when the user asks about Y or when working on Z.
disable-model-invocation: true
---

# Skill title

## Instructions

1. …
2. …

## Examples

Concrete input and output from the session.
```

Keep under 500 lines. Move long reference material to `reference.md`.

## Where to write

| Scope | Path |
|-------|------|
| Cursor user | `~/.cursor/skills/<name>/SKILL.md` |
| Cursor project | `.cursor/skills/<name>/SKILL.md` |
| Claude user | `~/.claude/skills/<name>/SKILL.md` |
| Claude project | `.claude/skills/<name>/SKILL.md` |
| Kitchen repo | `skills/<name>/SKILL.md` (canonical; link via `kitchen serve`) |

Never write to `~/.cursor/skills-cursor/` — Cursor built-in skills only.

## After creating

1. Tell the user the path written
2. Suggest `kitchen` pantry TUI to preview (`kitchen` in terminal)
3. If written in kitchen repo, suggest `kitchen serve` to link into their harness

## Quality bar

- **Specific triggers** — "Use when user mentions X" not "Use for general tasks"
- **Actionable steps** — commands, file paths, checks that worked here
- **No filler** — no "be helpful" or "consider best practices"
- **One skill, one job** — split if triggers diverge
