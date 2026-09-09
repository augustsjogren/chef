# Pi + Qwen Local Setup Summary

## Goal

Run **Qwen3.5-9B MLX 4-bit** as Pi coding agent on **M1 Pro 32GB**, bypassing Bionic/LM Studio context auto-fit issues. Direct **`mlx_vlm` server** on port **8080**.

---

## Inference stack

| Piece | Where |
|--------|--------|
| Model | `~/.lmstudio/models/lmstudio-community/Qwen3.5-9B-MLX-4bit` |
| Server script | `~/.pi/start-qwen-server.sh` |
| Venv | `~/.pi/mlx-server-venv` |
| Patch | `mlx_vlm/server/openai.py` — maps `developer` → `system` (Pi sends developer role) |

**Server flags:** `--max-kv-size 32768`, `--max-tokens 8192`, `--max-num-seqs 1`, `APC_ENABLED=1` (prefix cache across turns).

Bionic/LM Studio on `:1234` still fine for UI; Pi uses `:8080`.

---

## Pi config (`~/.pi/agent/settings.json`)

- **Provider:** `http:localhost:8080`
- **Thinking:** off for Qwen
- **Packages kept:** `pi-caveman`, `pi-fff`, `pi-token-speed`, `pi-local`
- **Removed:** `pi-lmstudio`, `@ollama/pi-web-search` (~200 prompt tokens saved)
- **Compaction:** `reserveTokens: 12288`, `keepRecentTokens: 16000`

Harmless startup 404s from pi-local probing `/v1/models/status` — `/v1/models` works.

---

## Extension: `cap-tool-output.ts`

Location: `extensions/cap-tool-output.ts` in this repo → `slop-chef serve link pi` → `~/.pi/agent/extensions/`.

| Hook | What it does |
|------|----------------|
| `tool_result` | Caps read/bash/grep/find/ff* output before model sees it |
| `context` | Redacts large **edit** args in history (30 lines / 1.5 KB per field) |
| `tool_call` | Blocks **edit** if total payload > 20 KB |

**Write:** no hooks — write context redaction and write blocking were removed. `edit`/`write` tool results also skip truncation.

### Tool result limits

| Tool | Lines | Bytes |
|------|-------|-------|
| read | 80 | 8 KB |
| bash | 120 | 12 KB |
| grep / find | 60 | 8 KB |
| fffind / ffgs / ffgrep | 40 | 6 KB |
| default | 100 | 10 KB |

---

## What worked

**Before:** `cached_tokens=0` every request; big reads blew prompt ~8.5k tokens; ~28s prefill per tool round.

**After APC + package trim + read caps:**

- Turn 1: ~27s cold prefill (~5.2k tokens)
- Turn 2+: `cached_tokens` matches prior prompt → **0.5–1s TTFT** when prefix stable
- Reads: ~6k tokens (~4s partial prefill), not 8.5k+

**Remaining bottleneck:** model sometimes does full-file **`write`** — long decode (~30–36s). Write hooks didn't help much; removed.

---

## Not done / later

- Lower `--max-tokens` on server to cap long writes
- Reapply `developer` → `system` patch after `mlx-vlm` reinstall
- Project rules nudging `edit` over `write` for local 9B
- `@arcanemachine/pi-read` — install blocked; local extension used instead

---

## Daily use

1. Stop Bionic models if you want headroom
2. Run `~/.pi/start-qwen-server.sh`
3. Pi with default provider `localhost:8080`
4. `/reload` or new session after extension changes
