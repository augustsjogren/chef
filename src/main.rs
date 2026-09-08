mod theme;
mod tty;
mod tui;
mod ui;

use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

use kitchen::config::{expand_path, harness_by_id, load_harnesses, repo_root};
use kitchen::item::{classify, items_for};
use kitchen::kitchen::Kitchen;
use kitchen::ops::{link_item, unlink_item};
use kitchen::stove::{cool, print_status, simmer, Burner};

#[derive(Parser)]
#[command(
    name = "kitchen",
    version,
    about = "Kitchen CLI — pantry, recipes, and the line.",
    after_help = "Examples:\n  kitchen cook\n  kitchen cook frontend Fix accessibility\n  kitchen prep\n  kitchen serve\n  kitchen stove simmer mlx"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Mise en place: build this repo and install `kitchen` on PATH
    Prep,
    /// Open the TUI, or put an agent on a ticket
    Cook {
        /// Agent name (spawned if new)
        agent: Option<String>,
        /// Task words after the name
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        task: Vec<String>,
        /// LET HIM COOK — hands off
        #[arg(short, long)]
        autonomous: bool,
    },
    /// Print who's on the line
    Status,
    /// Remove an agent from the roster
    Dismiss { agent: String },
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
    /// Run from repo source (`cargo run`) — no `kitchen prep` between edits
    Dev {
        /// Rebuild and rerun when sources change (needs `cargo install cargo-watch`)
        #[arg(long, short)]
        watch: bool,
        /// kitchen subcommand and args (default: cook TUI)
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
        None => run_tui("kitchen status"),
        Some(Cmd::Cook {
            agent,
            task,
            autonomous,
        }) => match agent {
            None if task.is_empty() => run_tui("kitchen status"),
            None => bail!(
                "need an agent name before the ticket — got: {}",
                task.join(" ")
            ),
            Some(agent) => {
                let ticket = if task.is_empty() {
                    None
                } else {
                    Some(task.join(" "))
                };
                let mut k = Kitchen::load()?;
                let id = k.cook_order(&agent, ticket.as_deref(), autonomous)?;
                let a = k
                    .agents
                    .iter()
                    .find(|a| a.id == id)
                    .context("missing after cook")?;
                println!(
                    "{}  {}  {}",
                    a.status.label(a.autonomous),
                    a.name,
                    a.task.as_deref().unwrap_or("Idle")
                );
                Ok(())
            }
        },
        Some(Cmd::Prep) => prep(),
        Some(Cmd::Status) => print_kitchen(),
        Some(Cmd::Dismiss { agent }) => dismiss_agent(&agent),
        Some(Cmd::Serve { project, cmd }) => serve(project, cmd),
        Some(Cmd::Stove { cmd }) => stove(cmd),
        Some(Cmd::Dev { watch, args }) => dev(watch, args),
    }
}

fn run_tui(hint: &str) -> Result<()> {
    if !tty::stdin_is_tty() {
        bail!("need a tty; try: {hint}");
    }
    ui::run()
}

fn prep() -> Result<()> {
    let root = repo_root();
    eprintln!("prep: {}", root.display());
    cargo(&root, &["build"])?;
    cargo(&root, &["install", "--path", ".", "--force"])?;
    eprintln!("prep: `kitchen` is on PATH (need ~/.cargo/bin in PATH)");
    eprintln!("prep: hacking? use `./dev` or `kitchen dev` — skips install");
    Ok(())
}

fn dev(watch: bool, args: Vec<String>) -> Result<()> {
    let root = repo_root();
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
                bail!("need a tty; try: kitchen serve status");
            }
            tui::run(project)
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

fn dismiss_agent(name: &str) -> Result<()> {
    let mut k = Kitchen::load()?;
    let idx = k
        .find_agent(name)
        .with_context(|| format!("no agent: {name}"))?;
    let removed = k.remove_agent(idx)?;
    println!("dismissed {removed}");
    Ok(())
}

fn print_kitchen() -> Result<()> {
    let k = Kitchen::load()?;
    let (skills, rules) = Kitchen::pantry()?;
    if k.agents.is_empty() {
        println!("kitchen empty  (stove's cold)");
    }
    for a in &k.agents {
        let task = a.task.as_deref().unwrap_or("Idle");
        println!(
            "{}  {:<16}  {:<14}  {task}",
            a.status.bullet(),
            a.name,
            a.status.label(a.autonomous)
        );
    }
    println!(
        "skills {}  rules {}  recipes {}",
        skills.len(),
        rules.len(),
        k.recipes.len()
    );
    Ok(())
}

fn print_serve_status(h: &kitchen::Harness, project: Option<&std::path::Path>) -> Result<()> {
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
        None => print_status(),
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
