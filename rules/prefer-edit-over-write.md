---
description: Prefer surgical edits over full-file writes on small local models (Pi + Qwen/Ollama). Use when context is tight, decode is slow, or the user runs a local 9B-class model.
alwaysApply: false
globs: ""
---

# Prefer edit over write

On small local models, full-file `write` dominates decode time and blows the context window. Default to minimal diffs.

- **Existing file:** use `edit` (or small patches). Never rewrite the whole file unless the user asked for a full rewrite.
- **New file:** `write` is fine when the file does not exist yet.
- **Exploring:** `read` with `offset`/`limit`, or narrow `grep` — not whole-file reads when you only need a section.
- **Large change:** several small `edit` calls beat one giant `write`.

```text
# bad on local 9B
write entire 400-line file → ~30s decode

# good
edit the 5 lines that changed → fast
```

If `edit` payload is blocked (extension limit), split into multiple smaller edits instead of switching to `write`.
