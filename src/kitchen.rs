use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::repo_root;
use crate::item::list_names;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Raw,
    Cooking,
    Cooked,
    Burnt,
}

impl Status {
    pub fn label(self, autonomous: bool) -> &'static str {
        match self {
            Self::Raw => "RAW",
            Self::Cooking if autonomous => "LET HIM COOK",
            Self::Cooking => "COOKING",
            Self::Cooked => "COOKED",
            Self::Burnt => "BURNT",
        }
    }

    pub fn glyph(self, autonomous: bool) -> &'static str {
        match self {
            Self::Raw => "·",
            Self::Cooking if autonomous => "🍳",
            Self::Cooking => "🔥",
            Self::Cooked => "✓",
            Self::Burnt => "💀",
        }
    }

    pub fn bullet(self) -> &'static str {
        match self {
            Self::Raw => "○",
            Self::Cooking => "●",
            Self::Cooked => "●",
            Self::Burnt => "✕",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub status: Status,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub recipe: Option<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default)]
    pub autonomous: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StateFile {
    #[serde(default)]
    agents: Vec<Agent>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RecipesFile {
    #[serde(default)]
    recipes: Vec<Recipe>,
}

#[derive(Debug, Clone)]
pub struct Kitchen {
    pub agents: Vec<Agent>,
    pub recipes: Vec<Recipe>,
}

impl Kitchen {
    pub fn load() -> Result<Self> {
        let state = read_json::<StateFile>(&state_path()).unwrap_or_default();
        let recipes = read_json::<RecipesFile>(&recipes_path()).unwrap_or_default();
        Ok(Self {
            agents: state.agents,
            recipes: recipes.recipes,
        })
    }

    pub fn save(&self) -> Result<()> {
        fs::create_dir_all(kitchen_dir())?;
        write_json(
            &state_path(),
            &StateFile {
                agents: self.agents.clone(),
            },
        )?;
        write_json(
            &recipes_path(),
            &RecipesFile {
                recipes: self.recipes.clone(),
            },
        )?;
        Ok(())
    }

    pub fn pantry() -> Result<(Vec<String>, Vec<String>)> {
        Ok((list_names("skills")?, list_names("rules")?))
    }

    /// One-line roster summary for the kitchen TUI header.
    pub fn line_summary(&self) -> String {
        if self.agents.is_empty() {
            return "stove's cold".into();
        }
        let mut raw = 0usize;
        let mut cooking = 0usize;
        let mut cooked = 0usize;
        let mut burnt = 0usize;
        for a in &self.agents {
            match a.status {
                Status::Raw => raw += 1,
                Status::Cooking => cooking += 1,
                Status::Cooked => cooked += 1,
                Status::Burnt => burnt += 1,
            }
        }
        let mut parts = Vec::new();
        if raw > 0 {
            parts.push(format!("{raw} raw"));
        }
        if cooking > 0 {
            parts.push(format!("{cooking} cooking"));
        }
        if cooked > 0 {
            parts.push(format!("{cooked} cooked"));
        }
        if burnt > 0 {
            parts.push(format!("{burnt} burnt"));
        }
        parts.join(" · ")
    }

    pub fn spawn(&mut self, name: &str, task: Option<String>) -> Result<String> {
        let name = name.trim();
        anyhow::ensure!(!name.is_empty(), "name required");
        let id = unique_id(&slug(name), &self.agents);
        let status = if task.as_ref().is_some_and(|t| !t.trim().is_empty()) {
            Status::Cooking
        } else {
            Status::Raw
        };
        let agent = Agent {
            id: id.clone(),
            name: name.to_string(),
            status,
            task: task.filter(|t| !t.trim().is_empty()),
            recipe: None,
            skills: Vec::new(),
            rules: Vec::new(),
            autonomous: false,
        };
        let line = match &agent.task {
            Some(t) => format!("spawned · {t}"),
            None => "spawned · idle".into(),
        };
        append_log(&id, &line)?;
        self.agents.push(agent);
        self.save()?;
        Ok(id)
    }

    pub fn assign_task(&mut self, idx: usize, task: &str) -> Result<()> {
        let task = task.trim();
        anyhow::ensure!(!task.is_empty(), "task required");
        let agent = self.agents.get_mut(idx).context("no agent")?;
        agent.task = Some(task.to_string());
        agent.status = Status::Cooking;
        append_log(&agent.id, &format!("ticket · {task}"))?;
        self.save()?;
        Ok(())
    }

    pub fn find_agent(&self, name_or_id: &str) -> Option<usize> {
        let q = name_or_id.trim();
        self.agents
            .iter()
            .position(|a| a.id == q || a.name.eq_ignore_ascii_case(q))
    }

    /// Put someone on a ticket. Spawns the agent if they are new.
    pub fn cook_order(
        &mut self,
        name: &str,
        task: Option<&str>,
        autonomous: bool,
    ) -> Result<String> {
        let name = name.trim();
        anyhow::ensure!(!name.is_empty(), "agent name required");
        let task = task.map(str::trim).filter(|t| !t.is_empty());

        let idx = if let Some(i) = self.find_agent(name) {
            i
        } else {
            let ticket = task.with_context(|| {
                format!("new cook needs a ticket — try: kitchen cook {name} <task>")
            })?;
            let id = self.spawn(name, Some(ticket.to_string()))?;
            let i = self.find_agent(&id).context("spawned but missing")?;
            self.apply_heat(i, autonomous)?;
            return Ok(self.agents[i].id.clone());
        };

        if let Some(ticket) = task {
            self.assign_task(idx, ticket)?;
        } else {
            let agent = self.agents.get_mut(idx).context("no agent")?;
            if agent.task.is_none() {
                anyhow::bail!("{name} has no ticket — try: kitchen cook {name} <task>");
            }
            agent.status = Status::Cooking;
            append_log(&agent.id, "back on the heat")?;
            self.save()?;
        }
        self.apply_heat(idx, autonomous)?;
        Ok(self.agents[idx].id.clone())
    }

    fn apply_heat(&mut self, idx: usize, autonomous: bool) -> Result<()> {
        let agent = self.agents.get_mut(idx).context("no agent")?;
        agent.status = Status::Cooking;
        agent.autonomous = autonomous;
        if autonomous {
            append_log(&agent.id, "let him cook")?;
            self.save()?;
        }
        Ok(())
    }

    pub fn kill(&mut self, idx: usize) -> Result<&'static str> {
        let agent = self.agents.get_mut(idx).context("no agent")?;
        match agent.status {
            Status::Cooking => {
                agent.status = Status::Raw;
                agent.autonomous = false;
                append_log(&agent.id, "pulled off the heat")?;
                self.save()?;
                Ok("stopped")
            }
            _ => Ok("not cooking"),
        }
    }

    /// Remove agent from the roster. Log file is kept in `.kitchen/logs/`.
    pub fn remove_agent(&mut self, idx: usize) -> Result<String> {
        let agent = self.agents.get(idx).context("no agent")?;
        let name = agent.name.clone();
        append_log(&agent.id, "dismissed from the line")?;
        self.agents.remove(idx);
        self.save()?;
        Ok(name)
    }

    pub fn plate(&mut self, idx: usize) -> Result<()> {
        let agent = self.agents.get_mut(idx).context("no agent")?;
        agent.status = Status::Cooked;
        agent.autonomous = false;
        append_log(&agent.id, "plated")?;
        self.save()?;
        Ok(())
    }

    pub fn burn(&mut self, idx: usize) -> Result<()> {
        let agent = self.agents.get_mut(idx).context("no agent")?;
        agent.status = Status::Burnt;
        agent.autonomous = false;
        append_log(&agent.id, "burnt")?;
        self.save()?;
        Ok(())
    }

    pub fn toggle_autonomous(&mut self, idx: usize) -> Result<()> {
        let agent = self.agents.get_mut(idx).context("no agent")?;
        if agent.task.is_none() {
            anyhow::bail!("give them a ticket first");
        }
        match agent.status {
            Status::Raw | Status::Cooked | Status::Burnt => {
                agent.status = Status::Cooking;
            }
            Status::Cooking => {}
        }
        agent.autonomous = !agent.autonomous;
        let msg = if agent.autonomous {
            "let him cook"
        } else {
            "hands back on"
        };
        append_log(&agent.id, msg)?;
        self.save()?;
        Ok(())
    }

    pub fn apply_recipe(&mut self, agent_idx: usize, recipe_idx: usize) -> Result<()> {
        let recipe = self.recipes.get(recipe_idx).cloned().context("no recipe")?;
        let agent = self.agents.get_mut(agent_idx).context("no agent")?;
        agent.recipe = Some(recipe.id.clone());
        agent.skills = recipe.skills.clone();
        agent.rules = recipe.rules.clone();
        append_log(&agent.id, &format!("recipe · {}", recipe.name))?;
        self.save()?;
        Ok(())
    }

    pub fn add_skill(&mut self, idx: usize, name: &str) -> Result<()> {
        let agent = self.agents.get_mut(idx).context("no agent")?;
        if !agent.skills.iter().any(|s| s == name) {
            agent.skills.push(name.to_string());
            append_log(&agent.id, &format!("skill + {name}"))?;
            self.save()?;
        }
        Ok(())
    }

    pub fn add_rule(&mut self, idx: usize, name: &str) -> Result<()> {
        let agent = self.agents.get_mut(idx).context("no agent")?;
        if !agent.rules.iter().any(|s| s == name) {
            agent.rules.push(name.to_string());
            append_log(&agent.id, &format!("rule + {name}"))?;
            self.save()?;
        }
        Ok(())
    }

    pub fn remove_skill(&mut self, idx: usize, name: &str) -> Result<()> {
        let agent = self.agents.get_mut(idx).context("no agent")?;
        let before = agent.skills.len();
        agent.skills.retain(|s| s != name);
        if agent.skills.len() < before {
            append_log(&agent.id, &format!("skill - {name}"))?;
            self.save()?;
        }
        Ok(())
    }

    pub fn remove_rule(&mut self, idx: usize, name: &str) -> Result<()> {
        let agent = self.agents.get_mut(idx).context("no agent")?;
        let before = agent.rules.len();
        agent.rules.retain(|s| s != name);
        if agent.rules.len() < before {
            append_log(&agent.id, &format!("rule - {name}"))?;
            self.save()?;
        }
        Ok(())
    }

    pub fn new_recipe(&mut self, name: &str, from_agent: Option<usize>) -> Result<()> {
        let name = name.trim();
        anyhow::ensure!(!name.is_empty(), "name required");
        let id = unique_recipe_id(&slug(name), &self.recipes);
        let (skills, rules) = match from_agent.and_then(|i| self.agents.get(i)) {
            Some(a) => (a.skills.clone(), a.rules.clone()),
            None => (Vec::new(), Vec::new()),
        };
        self.recipes.push(Recipe {
            id,
            name: name.to_string(),
            skills,
            rules,
        });
        self.save()?;
        Ok(())
    }

    pub fn delete_recipe(&mut self, idx: usize) -> Result<()> {
        anyhow::ensure!(idx < self.recipes.len(), "no recipe");
        self.recipes.remove(idx);
        self.save()?;
        Ok(())
    }
}

pub fn read_log(id: &str) -> String {
    fs::read_to_string(log_path(id)).unwrap_or_default()
}

#[cfg(test)]
thread_local! {
    static TEST_KITCHEN_DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(crate) fn set_test_kitchen_dir(dir: Option<PathBuf>) {
    TEST_KITCHEN_DIR.with(|slot| *slot.borrow_mut() = dir);
}

fn kitchen_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(dir) = TEST_KITCHEN_DIR
        .try_with(|slot| slot.borrow().clone())
        .ok()
        .flatten()
    {
        return dir;
    }
    repo_root().join(".kitchen")
}

fn state_path() -> PathBuf {
    kitchen_dir().join("state.json")
}

fn recipes_path() -> PathBuf {
    kitchen_dir().join("recipes.json")
}

fn log_path(id: &str) -> PathBuf {
    kitchen_dir().join("logs").join(format!("{id}.log"))
}

fn append_log(id: &str, line: &str) -> Result<()> {
    let dir = kitchen_dir().join("logs");
    fs::create_dir_all(&dir)?;
    let stamp = chrono::Local::now().format("%H:%M:%S");
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path(id))?;
    writeln!(f, "{stamp}  {line}")?;
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(value)?;
    fs::write(path, raw + "\n")?;
    Ok(())
}

pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if (c == '-' || c == '_' || c.is_whitespace())
            && !out.ends_with('-')
            && !out.is_empty()
        {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "cook".into()
    } else {
        out
    }
}

fn unique_id(base: &str, agents: &[Agent]) -> String {
    if !agents.iter().any(|a| a.id == base) {
        return base.to_string();
    }
    for n in 2..1000 {
        let id = format!("{base}-{n}");
        if !agents.iter().any(|a| a.id == id) {
            return id;
        }
    }
    format!("{base}-x")
}

fn unique_recipe_id(base: &str, recipes: &[Recipe]) -> String {
    if !recipes.iter().any(|r| r.id == base) {
        return base.to_string();
    }
    for n in 2..1000 {
        let id = format!("{base}-{n}");
        if !recipes.iter().any(|r| r.id == id) {
            return id;
        }
    }
    format!("{base}-x")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_strips_noise() {
        assert_eq!(slug("Front End!!"), "front-end");
        assert_eq!(slug("@@@"), "cook");
    }

    #[test]
    fn line_summary_counts_by_status() {
        let k = Kitchen {
            agents: vec![
                Agent {
                    id: "a".into(),
                    name: "a".into(),
                    status: Status::Cooking,
                    task: None,
                    recipe: None,
                    skills: vec![],
                    rules: vec![],
                    autonomous: false,
                },
                Agent {
                    id: "b".into(),
                    name: "b".into(),
                    status: Status::Cooked,
                    task: None,
                    recipe: None,
                    skills: vec![],
                    rules: vec![],
                    autonomous: false,
                },
            ],
            recipes: vec![],
        };
        assert_eq!(k.line_summary(), "1 cooking · 1 cooked");
        assert_eq!(
            Kitchen {
                agents: vec![],
                recipes: vec![],
            }
            .line_summary(),
            "stove's cold"
        );
    }

    #[test]
    fn remove_agent_drops_from_roster() {
        let tmp = tempfile::tempdir().unwrap();
        let kitchen_dir = tmp.path().join(".kitchen");
        fs::create_dir_all(&kitchen_dir).unwrap();
        set_test_kitchen_dir(Some(kitchen_dir));

        let mut k = Kitchen {
            agents: vec![Agent {
                id: "test".into(),
                name: "test".into(),
                status: Status::Raw,
                task: None,
                recipe: None,
                skills: vec![],
                rules: vec![],
                autonomous: false,
            }],
            recipes: vec![],
        };
        k.remove_agent(0).unwrap();
        assert!(k.agents.is_empty());

        set_test_kitchen_dir(None);
    }

    #[test]
    fn cooking_label_splits_autonomous() {
        assert_eq!(Status::Cooking.label(false), "COOKING");
        assert_eq!(Status::Cooking.label(true), "LET HIM COOK");
    }
}
