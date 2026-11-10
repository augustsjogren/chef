mod markdown;
mod pantry_ui;
mod serve_ui;
mod stove_ui;
mod theme;
mod tty;

use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

use chef::bootstrap;
use chef::config::{expand_path, harness_by_id, load_harnesses, tool_root};
use chef::import::{find_external_skill, import_skill};
use chef::item::{self, classify, items_for};
use chef::ops::{link_item, unlink_item};
use chef::stove::{cool, print_status, simmer, Burner};

#[derive(Parser)]
#[command(
    name = "chef",
    version,
    about = "chef — browse kitchen, serve, and stove.",
    after_help = "Examples:\n  chef\n  chef bootstrap\n  chef stove\n  chef serve\n  chef stove simmer mlx"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Mise en place: build this repo and install `chef` on PATH
    Prep,
    /// Browse skills, rules, and extensions
    #[command(name = "pantry")]
    Pantry,
    /// Print kitchen counts
    Status,
    /// Copy an external skill into kitchen
    ///
    /// Use --keep-source for a second copy, or --force to replace the external
    /// source with a symlink to the kitchen copy.
    Import {
        name: String,
        /// Pantry location label (`cursor-user`, `claude-user`, …)
        #[arg(long)]
        from: Option<String>,
        /// Keep the original copy instead of replacing it with a symlink
        #[arg(long)]
        keep_source: bool,
        /// Allow replacing external source directory with a symlink
        #[arg(long, conflicts_with = "keep_source")]
        force: bool,
    },
    /// Install chef + chef-add-skill into a harness skill dir
    Bootstrap {
        /// Harness id from harnesses/targets.json
        #[arg(long, default_value = "cursor-user")]
        harness: String,
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// Symlink skills/rules/extensions into Cursor / Claude / Pi
    Serve {
        /// Show planned link changes without modifying harness directories
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        project: Option<PathBuf>,
        #[command(subcommand)]
        cmd: Option<ServeCmd>,
    },
    /// Install persistent compact rendering for pi-fff
    SyncPiFff,
    /// Local model servers on the stove (MLX, Ollama)
    Stove {
        #[command(subcommand)]
        cmd: Option<StoveCmd>,
    },
    /// Run from repo source (`cargo run`) — no `chef prep` between edits
    Dev {
        /// Rebuild and rerun when sources change (needs `cargo install cargo-watch`)
        #[arg(long, short)]
        watch: bool,
        /// chef subcommand and args (default: pantry TUI)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
enum ServeCmd {
    Status,
    Harnesses,
    Link { harness: String },
    Unlink { harness: String },
    /// Link every given harness (default: all non-project harnesses) in one pass
    Sync {
        /// Harness ids to link (default: all non-project harnesses)
        harnesses: Vec<String>,
    },
}

#[derive(Subcommand)]
enum StoveCmd {
    /// Bring a burner up to heat (start server)
    Simmer { burner: String },
    /// Take a burner off the heat (stop server)
    Cool { burner: String },
}

/// Default SIGPIPE handling kills agents that pipe chef output into `head` and
/// friends with an ugly panic; restore the default terminate-on-pipe behavior.
#[cfg(unix)]
fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {}

fn main() -> Result<()> {
    reset_sigpipe();
    let cli = Cli::parse();
    match cli.cmd {
        None => run_tui_named("chef", pantry_ui::run),
        Some(Cmd::Pantry) => run_tui_named("chef pantry", pantry_ui::run),
        Some(Cmd::Prep) => prep(),
        Some(Cmd::Status) => print_pantry(),
        Some(Cmd::Import {
            name,
            from,
            keep_source,
            force,
        }) => cmd_import(&name, from.as_deref(), keep_source, force),
        Some(Cmd::Bootstrap { harness, project }) => {
            cmd_bootstrap(&harness, project.map(|p| expand_path(&p.to_string_lossy())))
        }
        Some(Cmd::Serve {
            dry_run,
            project,
            cmd,
        }) => serve(project, cmd, dry_run),
        Some(Cmd::SyncPiFff) => sync_pi_fff(),
        Some(Cmd::Stove { cmd }) => stove(cmd),
        Some(Cmd::Dev { watch, args }) => dev(watch, args),
    }
}

fn run_tui_named(hint: &str, run: fn() -> Result<()>) -> Result<()> {
    if !tty::stdin_is_tty() {
        bail!("need a tty; try: {hint}");
    }
    run()
}

fn prep() -> Result<()> {
    let root = tool_root();
    eprintln!("prep: {}", root.display());
    cargo(&root, &["build"])?;
    cargo(&root, &["install", "--path", ".", "--force"])?;
    eprintln!("prep: `chef` is on PATH (need ~/.cargo/bin in PATH)");
    eprintln!("prep: hacking? use `./dev` or `chef dev` — skips install");
    Ok(())
}

fn dev(watch: bool, args: Vec<String>) -> Result<()> {
    let root = tool_root();
    if watch {
        dev_watch(&root, &args)?;
    } else {
        dev_run(&root, &args)?;
    }
    Ok(())
}

fn dev_run(root: &std::path::Path, args: &[String]) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(root).args(["run", "--quiet", "--"]);
    if !args.is_empty() {
        cmd.args(args);
    }
    let status = cmd.status().context("cargo run")?;
    if !status.success() {
        bail!("cargo run failed");
    }
    Ok(())
}

fn dev_watch(root: &std::path::Path, args: &[String]) -> Result<()> {
    if !Command::new("cargo")
        .arg("watch")
        .arg("--version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        bail!("cargo-watch not installed — run: cargo install cargo-watch");
    }
    let x = if args.is_empty() {
        "run --quiet --".to_string()
    } else {
        format!("run --quiet -- {}", args.join(" "))
    };
    let status = Command::new("cargo")
        .current_dir(root)
        .args(["watch", "--quiet", "-x", &x])
        .status()
        .context("cargo watch")?;
    if !status.success() {
        bail!("cargo watch failed");
    }
    Ok(())
}

fn cargo(root: &std::path::Path, args: &[&str]) -> Result<()> {
    let status = Command::new("cargo")
        .current_dir(root)
        .args(args)
        .status()
        .context("cargo")?;
    if !status.success() {
        bail!("cargo {} failed", args.join(" "));
    }
    Ok(())
}

fn serve(project: Option<PathBuf>, cmd: Option<ServeCmd>, dry_run: bool) -> Result<()> {
    let project = project.map(|p| expand_path(&p.to_string_lossy()));
    match cmd {
        // Bare `serve` is a CLI report: same output as `serve status`.
        // Interactive linking lives in the `chef` TUI (serve tab).
        None | Some(ServeCmd::Status) => {
            for h in load_harnesses()? {
                print_serve_status(&h, project.as_deref())?;
                println!();
            }
            Ok(())
        }
        Some(ServeCmd::Harnesses) => {
            for h in load_harnesses()? {
                let tag = if h.project { " [project]" } else { "" };
                println!("{}\t{}{tag}", h.id, h.label);
            }
            Ok(())
        }
        Some(ServeCmd::Link { harness }) => apply(&harness, project.as_deref(), true, dry_run),
        Some(ServeCmd::Unlink { harness }) => apply(&harness, project.as_deref(), false, dry_run),
        Some(ServeCmd::Sync { harnesses }) => {
            let ids: Vec<String> = if harnesses.is_empty() {
                load_harnesses()?
                    .into_iter()
                    .filter(|h| !h.project)
                    .map(|h| h.id)
                    .collect()
            } else {
                harnesses
            };
            for id in ids {
                println!("# {id}");
                apply(&id, project.as_deref(), true, dry_run)?;
                println!();
            }
            Ok(())
        }
    }
}

fn print_pantry() -> Result<()> {
    println!(
        "skills {}  rules {}  extensions {}",
        item::list_kind("skills")?.len(),
        item::list_kind("rules")?.len(),
        item::list_kind("extensions")?.len()
    );
    Ok(())
}

fn cmd_import(name: &str, from: Option<&str>, keep_source: bool, force: bool) -> Result<()> {
    let (location, source) = find_external_skill(name, from)?;
    let result = import_skill(name, &source, &location, keep_source, force)?;
    println!("imported skills/{name} → {}", result.repo_path.display());
    if let Some(loc) = result.relinked {
        println!("relinked {loc} → repo (no duplicate copy)");
    } else if keep_source {
        println!("kept original at {}", source.display());
    }
    Ok(())
}

fn sync_pi_fff() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts/sync-pi-fff.sh");
    let status = std::process::Command::new("bash").arg(&script).status()?;
    if !status.success() {
        bail!("sync-pi-fff failed");
    }
    Ok(())
}

fn cmd_bootstrap(harness: &str, project: Option<PathBuf>) -> Result<()> {
    for (name, result) in bootstrap::bootstrap(harness, project.as_deref())? {
        println!("skills/{name}: {result}");
    }
    Ok(())
}

fn print_serve_status(h: &chef::Harness, project: Option<&std::path::Path>) -> Result<()> {
    println!("{} ({})", h.label, h.id);
    if h.project {
        match project {
            None => {
                println!("  project: (unset)  pass --project");
                return Ok(());
            }
            Some(p) => println!("  project: {}", p.display()),
        }
    }
    let rows = items_for(h, project)?;
    if rows.is_empty() {
        println!("  (no installable items)");
        return Ok(());
    }
    let width = rows.iter().map(|i| i.key().len()).max().unwrap_or(0);
    for item in rows {
        println!(
            "  {:<width$}  {:<8}  {}",
            item.key(),
            classify(&item),
            item.dest.display(),
            width = width
        );
    }
    Ok(())
}

fn stove(cmd: Option<StoveCmd>) -> Result<()> {
    match cmd {
        None => {
            if tty::stdin_is_tty() {
                stove_ui::run()
            } else {
                print_status()
            }
        }
        Some(StoveCmd::Simmer { burner }) => {
            println!("{}", simmer(Burner::parse(&burner)?)?);
            Ok(())
        }
        Some(StoveCmd::Cool { burner }) => {
            println!("{}", cool(Burner::parse(&burner)?)?);
            Ok(())
        }
    }
}

fn apply(id: &str, project: Option<&std::path::Path>, link: bool, dry_run: bool) -> Result<()> {
    let harness = harness_by_id(id)?;
    if harness.project && project.is_none() {
        bail!("{} needs --project", harness.id);
    }
    for item in items_for(&harness, project)? {
        if dry_run {
            println!(
                "{}: would {} → {}",
                item.key(),
                if link { "link" } else { "unlink" },
                item.dest.display()
            );
            continue;
        }
        let result = if link {
            link_item(&item)
        } else {
            unlink_item(&item)
        };
        println!("{}: {result}", item.key());
    }
    Ok(())
}
