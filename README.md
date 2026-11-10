# chef

<img src="assets/chef.png" alt="Pixel-art chef mascot" width="320">

*Skills in, good code out.*

`chef` is a personal Rust CLI/TUI for browsing a skill pantry, importing skills, linking them into AI coding harnesses, and running local models. It is less “enterprise platform” and more “small kitchen where the mise en place is unusually well organized.” This repository is a showcase of one working setup, not an actively supported general-purpose product.

Supported platforms: **macOS and Linux**. Windows is not currently supported because serving and importing use Unix symlinks and Unix process tools.

Skills do not live in this repository. They live in [kitchen](https://github.com/augustsjogren/kitchen), which `chef` discovers at `~/kitchen` by default. See [KITCHEN.md](KITCHEN.md).

| Repository | Role |
|---|---|
| [augustsjogren/kitchen](https://github.com/augustsjogren/kitchen) | Skills, rules, and extensions |
| [augustsjogren/chef](https://github.com/augustsjogren/chef) | CLI and TUI in this repository |

## Requirements

Bring an appetite for terminal interfaces. The kitchen itself is optional; `chef` will tell you where to find one.

- macOS or Linux
- Git
- Rust and Cargo ([install Rust with rustup](https://rustup.rs/))
- Optional: Cursor, Claude, Pi, or any tool reading `~/.agents/skills` for `chef serve`
- Optional: Ollama or MLX for local-model commands

## Install

First, put on your terminal apron:

Clone this repository, then run the installer:

```bash
git clone https://github.com/augustsjogren/chef.git
cd chef
./install
```

Preview installer actions without changing files:

```bash
./install --dry-run
```

Skip shell profile changes:

```bash
./install --no-shell-modification
```

The installer builds `chef`, installs it through Cargo, and adds Cargo's bin directory to your shell startup file when needed. It does not install Rust or modify system packages.

Clone the skills repository and start using `chef`:

```bash
git clone https://github.com/augustsjogren/kitchen.git ~/kitchen
chef bootstrap
chef serve
chef
```

After changing this repository, run `chef prep` to rebuild and reinstall the binary.

## Showcase demo

A typical first service looks like this:

```text
$ chef status
skills 17  rules 2  extensions 1

$ chef serve --dry-run link cursor-user
skills/caveman: would link → /Users/you/.cursor/skills/caveman
…

$ chef serve sync
skills/caveman: linked
…

$ chef serve
cursor-user (cursor-user)
  skills/caveman            linked     /Users/you/.cursor/skills/caveman
  skills/caveman-compress   linked     /Users/you/.cursor/skills/caveman-compress
  skills/chef               linked     /Users/you/.cursor/skills/chef
  …
```

`chef` then opens the browser: tabbed skills · rules · extensions · serve panes with a preview pane, `/` filtering, and confirmations before anything moves.

The exact counts and harnesses depend on your `kitchen` repository. The TUI is the main dining room; CLI commands are the prep cooks making setup and linking scriptable.

## Commands

The binary is **`chef`**. Running it without a subcommand opens the browse TUI. No reservation required.

| Command | What |
|---|---|
| `chef` / `chef pantry` | Browse and import kitchen content in one TUI |
| `chef import <name> --keep-source` | Import an external skill and keep original copy |
| `chef import <name> --force` | Import and replace original source with a symlink |
| `chef bootstrap` | Install meta-skills into a harness |
| `chef serve` | Harness link report (agent-friendly; same as `chef serve status`) |
| `chef serve --dry-run link <id>` | Preview links without changing harness directories |
| `chef stove` | Manage local model servers |
| `chef status` | Print kitchen counts |
| `chef prep` | Rebuild and reinstall this CLI |

## The browse TUI

`chef` opens one screen with four tabs: **skills · rules · extensions · serve**. Press `?` for the full key list.

- **Browse** `h/l`/`Tab` between tabs, `j/k` to move, `a` toggles repo-only vs. all sources, `PgUp/PgDn` scrolls the preview pane.
- **Filter** with `/`, type to narrow the list live, `Esc` clears; the title shows the active pattern.
- **Import** with `i` (skills tab + all sources): a `Y/N` popup confirms before a skill is copied into the kitchen and its external source replaced by a symlink.
- **Serve tab** lists the harnesses from `harnesses/targets.json`, grouped by tool family. `Enter` opens a harness, `space`/`a` select items, and `w` links while `u` unlinks — both operate on the selection; the status row shows what would happen (`2 to link · 1 to unlink`). Unlink always asks first, and mis-pointed (foreign) links can be removed and healed with a fresh `w`.
- Item rows carry a colored link status (`linked`, `unlinked`, `foreign`, `blocked`); `blocked` means a real file/directory occupies the destination path and chef refuses to overwrite it.
- `q` quits. For scripts and agents, `chef serve` never opens a screen: bare `chef serve` prints the link report, and `link`, `unlink`, and `sync` are non-interactive subcommands.

Run `chef <command> --help` for CLI options.

## Personal assumptions

This setup reflects one developer's workflow:

- `kitchen` is a separate personal repository at `~/kitchen`.
- Cursor, Claude, Pi, and a generic `~/.agents/skills` directory are the harness targets.
- Unix symlinks are preferred over duplicated skill files.
- Local model workflows use Ollama, MLX, LM Studio, and Pi-specific scripts where configured.
- `~/.chef/` is personal runtime state, not portable project configuration.
- Harness paths and model commands may need edits for another machine.

`chef` is intentionally opinionated. It has a favorite knife and strong feelings about mise en place. Fork or adapt `harnesses/targets.json` and local-model configuration rather than expecting zero-configuration support for every environment.

## Configuration and state

| Variable | Default | Purpose |
|---|---|---|
| `CHEF_KITCHEN` | `~/kitchen` | Override kitchen location |

Runtime state is stored in `~/.chef/`. This includes model state and logs. Kitchen content is not copied into this repository.

## Troubleshooting

- If `chef` is not found after installation, open a new shell or source your shell profile.
- If no kitchen is found, clone `kitchen` to `~/kitchen` or set `CHEF_KITCHEN`.
- If `chef serve` cannot create links, verify that destination directories are writable and that your platform supports Unix symlinks.
- Local-model commands require their respective model runtime and binaries to be installed separately.
- `chef import <name>` refuses destructive relinking unless you pass `--force` or choose `--keep-source`.

## Development

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## License

MIT. See [LICENSE](LICENSE).
