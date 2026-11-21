# Claude Code

| Kind | This repo | Typical load path |
|------|-----------|-------------------|
| Skill | `skills/<name>/` | `~/.claude/skills/<name>/` or project `.claude/skills/<name>/` |
| Rule | `rules/<name>.md` | `~/.claude/rules/<name>.md`, imported by a managed block in `~/.claude/CLAUDE.md` |

Install: copy or symlink the skill directory. Keep `SKILL.md` at the skill root.

## Rules

`chef serve link claude-user` (or `chef serve sync`) symlinks each kitchen
rule into `~/.claude/rules/` and rewrites a marker-delimited block of
`@~/.claude/rules/<name>.md` imports in `~/.claude/CLAUDE.md`. Only the
block between `<!-- chef:begin -->` and `<!-- chef:end -->` is managed;
manual notes outside the markers are preserved.

For project scope (`claude-project`, needs `--project`), the same happens
into `<project>/.claude/rules/` with the import block in `<project>/CLAUDE.md`.

Claude Code ≥ 2.1.277 also reads a repo's `AGENTS.md` when no `CLAUDE.md`
exists, so project instructions can stay in one file.
