# kitchen

Personal pantry for agent skills, rules, extensions, and harness adapters — with a cooking-themed TUI.

Canonical definitions live by **kind**. Harness-specific install notes live under `harnesses/`. Do not fork the same skill into Cursor/Claude/Pi copies unless a harness truly cannot share the file.

| Path | What goes here |
|------|----------------|
| `skills/` | Portable Agent Skills (`SKILL.md` directories) |
| `rules/` | Persistent guidance (always-on or glob-scoped) |
| `extensions/` | Runtime hooks (Pi TypeScript, scripts, etc.) |
| `harnesses/` | How to install/symlink a kind into Cursor, Claude, Pi, … |
| `.kitchen/` | Local stove + chat logs |

See [`AGENTS.md`](AGENTS.md) for conventions when editing this repo.

CI runs `cargo fmt --check`, `cargo clippy`, and `cargo test` on push/PR.

## Install

```bash
./install
```

Installs rustup if needed, adds `~/.cargo/env` to your shell profile, builds `kitchen`, and puts it on `~/.cargo/bin`. Open a **new terminal** (or `source ~/.zshrc`), then:

```bash
kitchen                               # pantry TUI
kitchen chat                          # plain local-model chat
kitchen stove                         # burner TUI
kitchen serve                         # symlink pantry into Cursor / Claude / Pi
```

After pulling code: `kitchen prep` (rebuild + reinstall). Stove and chat state live in `.kitchen/` (local).

**Hacking this repo:** skip `prep` between edits — run `./dev`, `./dev stove`, `./dev chat`, or `./dev serve` (`cargo run` rebuilds only when needed). Auto-rebuild on save: `kitchen dev -w` (install [cargo-watch](https://github.com/watchexec/cargo-watch) once).

Set `KITCHEN_ROOT` to point at this repo if you run `kitchen` from elsewhere.

## CLI

| Command | What |
|---------|------|
| `kitchen` / `kitchen pantry` | Browse skills, rules, extensions |
| `kitchen stove` | Burner TUI (status if not a tty) |
| `kitchen stove simmer mlx` | Start MLX server (`:8080`) |
| `kitchen stove simmer ollama` | Start Ollama (`:11434`) |
| `kitchen stove cool mlx` | Stop MLX server |
| `kitchen stove cool ollama` | Stop Ollama |
| `kitchen serve` | Harness symlink TUI |
| `kitchen serve link <harness>` | Link all pantry items to one harness |
| `kitchen chat` | Plain chat with a local model (warms the stove) |
| `kitchen status` | Print pantry counts |
| `kitchen prep` | `cargo build` + `cargo install --path .` |
| `kitchen dev [args…]` | Run from source (`cargo run`) — no install step |
| `kitchen dev -w` | Auto-rebuild on save (needs `cargo-watch`) |

## TUI

**Pantry** (`kitchen`): tab through skills, rules, extensions. Read the file in the preview pane. Link into Cursor / Claude / Pi with `kitchen serve`.

**Stove** (`kitchen stove`): simmer / cool MLX and Ollama. Model catalog for the selected burner. Chat uses this catalog. Keys: `[enter]`/`[s]` simmer · `[c]` cool · `[r]` refresh · `[q]` quit.

**Serve** (`kitchen serve`): pick a harness, link or unlink pantry items.

**Chat** (`kitchen chat`): naked thread with a model from the stove catalog. First send warms MLX. Transcripts in `.kitchen/chats/`. Keys: enter send · esc stop · `^o` model · `^n` new · `^q` quit. No skills, no tools.

## Pantry (bundled)

| Kind | Name | Purpose |
|------|------|---------|
| Skill | `kitchen-repo` | How to work in this repo |
| Skill | `local-pi-qwen` | Pi + Qwen MLX on `:8080` |
| Rule | `prefer-edit-over-write` | Small local models: edit not write |
| Rule | `kitchen-repo` | Repo conventions (always-on here) |
| Extension | `cap-tool-output.ts` | Pi tool output caps — `kitchen serve link pi` |

Local Pi + Qwen stack notes: [`harnesses/pi/qwen-local.md`](harnesses/pi/qwen-local.md).
