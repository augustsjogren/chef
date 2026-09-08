# harnesses

Adapters only. Canonical skill/rule/extension text stays in the kind folders.

Each subdirectory documents:

1. Where that harness loads skills, rules, and extensions
2. How to symlink or copy from this repo
3. Machine- or model-specific notes (context caps, thinking off, ports)

Current: `cursor/`, `claude/`, `pi/`. Dest paths: `targets.json`. Run `kitchen serve` to symlink.
