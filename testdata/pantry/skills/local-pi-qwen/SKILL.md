---
name: local-pi-qwen
description: Run Pi against Qwen MLX on localhost with tight context and fast prefix cache. Use when working with Pi, local Qwen, mlx_vlm on port 8080, or the user's M1 local coding agent stack.
disable-model-invocation: true
---

# Local Pi + Qwen (MLX)

## Stack

| Piece | Location |
|-------|----------|
| Model | `~/.lmstudio/models/lmstudio-community/Qwen3.5-9B-MLX-4bit` |
| Server | `~/.pi/start-qwen-server.sh` → `mlx_vlm` on **:8080** |
| Pi provider | `http:localhost:8080` in `~/.pi/agent/settings.json` |
| Thinking | off for Qwen |

Server flags that matter: `--max-kv-size 32768`, `APC_ENABLED=1` (prefix cache across turns).

After `mlx-vlm` reinstall, reapply `developer` → `system` patch in `mlx_vlm/server/openai.py` (Pi sends `developer` role).

## Pi packages (kept)

`pi-caveman`, `@ff-labs/pi-fff`, `pi-token-speed`, `@monroewilliams/pi-local`

Removed for token savings: `pi-lmstudio`, `@ollama/pi-web-search`.

Compaction: `reserveTokens: 12288`, `keepRecentTokens: 16000`.

## Extension

Install from this repo:

```bash
kitchen serve
# in TUI: Pi harness → link cap-tool-output.ts
```

Or symlink: `extensions/cap-tool-output.ts` → `~/.pi/agent/extensions/`. `/reload` in Pi after changes.

Caps read/bash/grep output before the model sees it; redacts large `edit` args in history; blocks `edit` payloads over 20 KB.

## Daily

1. `~/.pi/start-qwen-server.sh`
2. Pi (default provider `:8080`)
3. Prefer `edit` over full-file `write` — long writes are the main decode bottleneck

Full notes: `harnesses/pi/qwen-local.md` in the kitchen repo.
