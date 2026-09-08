mod tty;
mod tui;
mod ui;

use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

use cook::config::{expand_path, harness_by_id, load_harnesses, repo_root};
use cook::item::{classify, items_for};
use cook::kitchen::Kitchen;
use cook::ops::{link_item, unlink_item};

#[derive(Parser)]
#[command(
    name = "kitchen",
    about = "One CLI for the line: pantry, recipes, agents.",
    after_help = "Examples:\n  kitchen cook\n  kitchen cook frontend Fix accessibility\n  kitchen prep\n  kitchen serve"
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
}

#[derive(Subcommand)]
enum ServeCmd {
    Status,
    Harnesses,
    Link { harness: String },
    Unlink { harness: String },
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

fn print_serve_status(h: &cook::Harness, project: Option<&std::path::Path>) -> Result<()> {
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
