# extensions

Runtime hooks that are not skills or rules: Pi `*.ts` extensions, wrapper scripts, output caps, etc.

| File | Purpose |
|------|---------|
| `cap-tool-output.ts` | Cap read/bash/grep output; redact large edit args; block huge edits (Pi) |

Install: `kitchen serve link pi` (symlink into `~/.pi/agent/extensions/`).

Do not commit machine-local paths as the only copy of logic. Notes about a machine can live under `harnesses/<name>/`.
