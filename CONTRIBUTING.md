# Contributing

Thanks for helping improve `chef`.

## Before opening a change

- Check existing issues and pull requests.
- Keep changes focused.
- Update README or command help when behavior changes.
- Do not commit personal kitchen content, runtime state, model logs, or credentials.

## Local checks

Run from repository root:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo package --locked
```

`chef` currently targets macOS and Linux. Changes involving symlinks, process management, harness paths, or local model runtimes should include platform notes and tests where practical.

## Pull requests

Describe what changed, why, and how it was tested. Include screenshots for visible TUI changes. Keep generated files and unrelated formatting changes out of the pull request.

## Reporting bugs

Include OS, Rust version, `chef --version`, command arguments, relevant environment variables, and the complete error output. Remove private paths, skill content, tokens, and other sensitive data first.
