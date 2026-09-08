# rules

Persistent guidance, one concern per file. Keep each rule short (aim under 50 lines of body).

Canonical files here are markdown. Harness adapters map them:

- Cursor: `.cursor/rules/<name>.mdc` (`description`, `globs`, `alwaysApply`)
- Claude: `CLAUDE.md` or `.claude/` project files
- Pi: project `AGENTS.md` / package prompts — see `harnesses/pi/`

Start from `_template.md`. Filename = kebab-case concern (`prefer-edit-over-write.md`).
