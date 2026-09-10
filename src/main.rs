mod chat_ui;
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
use chef::import::{find_external_skill, import_skill};
use chef::config::{expand_path, harness_by_id, load_harnesses, tool_root};
use chef::item::{self, classify, items_for};
use chef::ops::{link_item, unlink_item};
use chef::stove::{cool, print_status, simmer, Burner};

#[derive(Parser)]
#[command(
    name = "chef",
    version,
    about = "chef — browse kitchen, serve, stove, and chat.",
    after_help = "Examples:\n  chef\n  chef bootstrap\n  chef stove\n  chef serve\n  chef chat\n  chef stove simmer mlx"
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
    /// Plain chat with a local model on the stove
    Chat,
    /// Print kitchen counts
    Status,
    /// Copy an external skill into kitchen (relinks source by default)
    Import {
        name: String,
        /// Pantry location label (`cursor-user`, `claude-user`, …)
        #[arg(long)]
        from: Option<String>,
        /// Keep the original copy instead of replacing it with a symlink
        #[arg(long)]
        keep_source: bool,
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
        #[arg(long)]
        project: Option<PathBuf>,
        #[command(subcommand)]
        cmd: Option<ServeCmd>,
    },
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
}

#[derive(Subcommand)]
enum StoveCmd {
    /// Bring a burner up to heat (start server)
    Simmer { burner: String },
    /// Take a burner off the heat (stop server)
    Cool { burner: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        None => run_tui_named("chef", pantry_ui::run),
        Some(Cmd::Pantry) => run_tui_named("chef pantry", pantry_ui::run),
        Some(Cmd::Chat) => run_tui_named("chef chat", chat_ui::run),
        Some(Cmd::Prep) => prep(),
        Some(Cmd::Status) => print_pantry(),
        Some(Cmd::Import {
            name,
            from,
            keep_source,
        }) => cmd_import(&name, from.as_deref(), keep_source),
        Some(Cmd::Bootstrap { harness, project }) => {
            cmd_bootstrap(&harness, project.map(|p| expand_path(&p.to_string_lossy())))
        }
        Some(Cmd::Serve { project, cmd }) => serve(project, cmd),
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

fn serve(project: Option<PathBuf>, cmd: Option<ServeCmd>) -> Result<()> {
    let project = project.map(|p| expand_path(&p.to_string_lossy()));
    match cmd {
        None => {
            if !tty::stdin_is_tty() {
                bail!("need a tty; try: chef serve status");
            }
            serve_ui::run(project)
        }
        Some(ServeCmd::Harnesses) => {
            for h in load_harnesses()? {
                let tag = if h.project { " [project]" } else { "" };
                println!("{}\t{}{tag}", h.id, h.label);
            }
            Ok(())
        }
        Some(ServeCmd::Status) => {
            for h in load_harnesses()? {
                print_serve_status(&h, project.as_deref())?;
                println!();
            }
            Ok(())
        }
        Some(ServeCmd::Link { harness }) => apply(&harness, project.as_deref(), true),
        Some(ServeCmd::Unlink { harness }) => apply(&harness, project.as_deref(), false),
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

fn cmd_import(name: &str, from: Option<&str>, keep_source: bool) -> Result<()> {
    let (location, source) = find_external_skill(name, from)?;
    let result = import_skill(name, &source, &location, keep_source)?;
    println!("imported skills/{name} → {}", result.repo_path.display());
    if let Some(loc) = result.relinked {
        println!("relinked {loc} → repo (no duplicate copy)");
    } else if keep_source {
        println!("kept original at {}", source.display());
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

fn apply(id: &str, project: Option<&std::path::Path>, link: bool) -> Result<()> {
    let harness = harness_by_id(id)?;
    if harness.project && project.is_none() {
        bail!("{} needs --project", harness.id);
    }
    for item in items_for(&harness, project)? {
        let result = if link {
            link_item(&item)
        } else {
            unlink_item(&item)
        };
        println!("{}: {result}", item.key());
    }
    Ok(())
}
