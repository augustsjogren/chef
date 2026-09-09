# kitchen

Personal pantry for agent skills, rules, extensions, and harness adapters — with a cooking-themed TUI.

Canonical definitions live by **kind**. Harness-specific install notes live under `harnesses/`. Do not fork the same skill into Cursor/Claude/Pi copies unless a harness truly cannot share the file.

| Path | What goes here |
|------|----------------|
| `skills/` | Portable Agent Skills (`SKILL.md` directories) |
| `rules/` | Persistent guidance (always-on or glob-scoped) |
| `extensions/` | Runtime hooks (Pi TypeScript, scripts, etc.) |
| `harnesses/` | How to install/symlink a kind into Cursor, Claude, Pi, … |
| `.kitchen/` | Runtime: recipes (committed), agent state + logs (local) |

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
kitchen cook                          # roster TUI (frozen)
kitchen cook frontend Fix a11y        # spawn/assign + COOKING
```

After pulling code: `kitchen prep` (rebuild + reinstall). Agent state lives in `.kitchen/` (local).

**Hacking this repo:** skip `prep` between edits — run `./dev`, `./dev stove`, `./dev chat`, or `kitchen dev cook` (`cargo run` rebuilds only when needed). Auto-rebuild on save: `kitchen dev -w` (install [cargo-watch](https://github.com/watchexec/cargo-watch) once).

Set `KITCHEN_ROOT` to point at this repo if you run `kitchen` from elsewhere.

## CLI

| Command | What |
|---------|------|
| `kitchen` / `kitchen pantry` | Browse skills, rules, extensions, recipes |
| `kitchen stove` | Burner TUI (status if not a tty) |
| `kitchen stove simmer mlx` | Start MLX server (`:8080`) |
| `kitchen stove simmer ollama` | Start Ollama (`:11434`) |
| `kitchen stove cool mlx` | Stop MLX server |
| `kitchen stove cool ollama` | Stop Ollama |
| `kitchen serve` | Harness symlink TUI |
| `kitchen serve link <harness>` | Link all pantry items to one harness |
| `kitchen chat` | Plain chat with a local model (warms the stove) |
| `kitchen cook` | Roster TUI (frozen) |
| `kitchen cook <agent> <task>` | Spawn or assign a **ticket** and start COOKING |
| `kitchen cook <agent> <task> -a` | LET HIM COOK — dispatch ticket to MLX (raw HTTP) |
| `kitchen status` | Print roster + pantry counts |
| `kitchen dismiss <agent>` | Remove agent from roster (logs kept) |
| `kitchen prep` | `cargo build` + `cargo install --path .` |
| `kitchen dev [args…]` | Run from source (`cargo run`) — no install step |
| `kitchen dev -w` | Auto-rebuild on save (needs `cargo-watch`) |

## TUI

**Pantry** (`kitchen`): tab through skills, rules, extensions, recipes. Read the file in the preview pane. Link into Cursor / Claude / Pi with `kitchen serve`.

**Stove** (`kitchen stove`): simmer / cool MLX and Ollama. Model catalog for the selected burner. Chat uses this catalog.

**Serve** (`kitchen serve`): pick a harness, link or unlink pantry items.

**Chat** (`kitchen chat`): naked thread with a model from the stove catalog. First send warms MLX. Transcripts in `.kitchen/chats/`. Keys: enter send · esc stop · `^o` model · `^n` new · `^q` quit. No skills, no tools.

**Cook** (`kitchen cook`, frozen): roster of fake cooks. A **ticket** is the task string — free text like `fix login page`. Required before `[m]` let him cook.

| Status | Meaning |
|--------|---------|
| `RAW` | on roster, no work started |
| `COOKING` | working |
| `LET HIM COOK` | autonomous — ticket dispatches to MLX |
| `COOKED` | finished |
| `BURNT` | failed |

Keys: `[enter]` inspect · `[s]` spawn · `[t]` ticket · `[m]` let him cook · `[d]` stop (cancels in-flight dispatch) · `[D]` / Delete dismiss · `[c]` plate · `[x]` burnt · `[r]` recipes · `[p]` pantry · `[o]` stove · `[q]` quit. `j`/`k` move. Pantry: `[a]` attach · `[-]` detach. Stove: `[enter]`/`[s]` simmer · `[c]` cool · `[r]` refresh.

**Autonomous mode** (`-a` or `[m]`): kitchen warms MLX on the stove if needed, POSTs the ticket to `/v1/chat/completions`, and logs the reply under `.kitchen/logs/<agent-id>.log`. Inspect the agent to read it. Stop with `[d]` while dispatching.

## Pantry (bundled)

| Kind | Name | Purpose |
|------|------|---------|
| Skill | `kitchen-repo` | How to work in this repo |
| Skill | `local-pi-qwen` | Pi + Qwen MLX on `:8080` |
| Rule | `prefer-edit-over-write` | Small local models: edit not write |
| Rule | `kitchen-repo` | Repo conventions (always-on here) |
| Extension | `cap-tool-output.ts` | Pi tool output caps — `kitchen serve link pi` |

Recipe **Local Qwen 9B** (pantry → recipes) bundles the Pi skill + edit rule.

Local Pi + Qwen stack notes: [`harnesses/pi/qwen-local.md`](harnesses/pi/qwen-local.md).
