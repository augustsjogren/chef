# Pi

Pi loads skills from `~/.pi/agent/skills/`, plus packages and extensions. `chef serve` can link skills, rules, and extensions into Pi.

| Kind | This repo | Pi load path |
|------|-----------|--------------|
| Extension | `extensions/` | `~/.pi/agent/extensions/` |
| Skill | `skills/` | `~/.pi/agent/skills/` |
| Rule | `rules/` | `~/.pi/agent/rules/`, inlined into the system prompt by the `kitchen-rules` extension |

## Wire skills, rules, and extensions

```bash
chef serve
```

Pick **Pi** in the TUI, select skills, rules, or extensions, press `w` to link (from the harnesses screen, `S` links everything everywhere).

Or CLI:

```bash
chef serve link pi
```

Skills are symlinked into `~/.pi/agent/skills/`; extensions are symlinked into `~/.pi/agent/extensions/`. Restart Pi after skill changes; run `/reload` after extension changes.

## Rules

`chef serve link pi` symlinks each kitchen rule into `~/.pi/agent/rules/`.
The bundled `kitchen-rules` extension (`~/.pi/agent/extensions/kitchen-rules.ts`)
reads that directory at every run start and injects the rules as a
`kitchen_rules` system prompt section, skipping Cursor-style frontmatter.
Add or edit a rule in the kitchen, run `chef serve`, done.

Project-level instructions still go in the repo's `AGENTS.md` — Pi reads
those natively.
