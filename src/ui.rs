//! Kitchen TUI. Cooking words are labels; navigation stays boring and fast.

use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};

use kitchen::kitchen::{read_log, Kitchen, Status};
use kitchen::stove::{self, Burner, BurnerStatus, Heat};

use crate::theme;

const EMPTY_KITCHEN: [&str; 6] = [
    "stove's cold.  [s] spawn someone.",
    "nothing cooking.  [s] spawn someone.",
    "kitchen's empty.",
    "no one's cooking.",
    "we got no cooks.",
    "someone get in here.  [s] spawn someone.",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Kitchen,
    Inspect,
    Recipes,
    Pantry,
    Stove,
}

enum Prompt {
    SpawnName,
    SpawnTask { name: String },
    AssignTask,
    RecipeName,
}

enum PantryCol {
    Skills,
    Rules,
}

struct App {
    kitchen: Kitchen,
    screen: Screen,
    agent_list: ListState,
    recipe_list: ListState,
    skill_list: ListState,
    rule_list: ListState,
    burner_list: ListState,
    stove: Vec<BurnerStatus>,
    pantry_col: PantryCol,
    skills: Vec<String>,
    rules: Vec<String>,
    prompt: Option<Prompt>,
    input: String,
    flash: String,
}

impl App {
    fn new() -> Result<Self> {
        let kitchen = Kitchen::load()?;
        let (skills, rules) = Kitchen::pantry()?;
        let mut agent_list = ListState::default();
        if !kitchen.agents.is_empty() {
            agent_list.select(Some(0));
        }
        let mut recipe_list = ListState::default();
        if !kitchen.recipes.is_empty() {
            recipe_list.select(Some(0));
        }
        let mut skill_list = ListState::default();
        if !skills.is_empty() {
            skill_list.select(Some(0));
        }
        let mut rule_list = ListState::default();
        if !rules.is_empty() {
            rule_list.select(Some(0));
        }
        let stove = stove::status_all()?;
        let mut burner_list = ListState::default();
        if !stove.is_empty() {
            burner_list.select(Some(0));
        }
        Ok(Self {
            kitchen,
            screen: Screen::Kitchen,
            agent_list,
            recipe_list,
            skill_list,
            rule_list,
            burner_list,
            stove,
            pantry_col: PantryCol::Skills,
            skills,
            rules,
            prompt: None,
            input: String::new(),
            flash: String::new(),
        })
    }

    fn agent_idx(&self) -> Option<usize> {
        self.agent_list.selected()
    }

    fn recipe_idx(&self) -> Option<usize> {
        self.recipe_list.selected()
    }

    fn select_agent(&mut self, idx: usize) {
        if self.kitchen.agents.is_empty() {
            self.agent_list.select(None);
            return;
        }
        let idx = idx.min(self.kitchen.agents.len() - 1);
        self.agent_list.select(Some(idx));
    }

    fn select_recipe(&mut self, idx: usize) {
        if self.kitchen.recipes.is_empty() {
            self.recipe_list.select(None);
            return;
        }
        let idx = idx.min(self.kitchen.recipes.len() - 1);
        self.recipe_list.select(Some(idx));
    }

    fn refresh_stove(&mut self) -> Result<()> {
        self.stove = stove::status_all()?;
        if self.stove.is_empty() {
            self.burner_list.select(None);
        } else if self.burner_list.selected().is_none() {
            self.burner_list.select(Some(0));
        } else {
            let i = self
                .burner_list
                .selected()
                .unwrap_or(0)
                .min(self.stove.len() - 1);
            self.burner_list.select(Some(i));
        }
        Ok(())
    }

    fn burner_idx(&self) -> Option<usize> {
        self.burner_list.selected()
    }

    fn selected_burner(&self) -> Option<Burner> {
        self.burner_idx()
            .and_then(|i| self.stove.get(i))
            .map(|s| s.burner)
    }

    fn select_burner(&mut self, idx: usize) {
        if self.stove.is_empty() {
            self.burner_list.select(None);
            return;
        }
        let idx = idx.min(self.stove.len() - 1);
        self.burner_list.select(Some(idx));
    }

    fn refresh_pantry(&mut self) -> Result<()> {
        let (skills, rules) = Kitchen::pantry()?;
        self.skills = skills;
        self.rules = rules;
        Ok(())
    }

    fn remove_agent(&mut self, idx: usize) -> Result<()> {
        let name = self.kitchen.remove_agent(idx)?;
        if self.kitchen.agents.is_empty() {
            self.agent_list.select(None);
        } else {
            self.select_agent(idx.min(self.kitchen.agents.len() - 1));
        }
        if self.screen == Screen::Inspect {
            self.screen = Screen::Kitchen;
        }
        self.flash = format!("{name} dismissed");
        Ok(())
    }

    fn stop_agent(&mut self, idx: usize) {
        match self.kitchen.kill(idx) {
            Ok("stopped") => self.flash = "off the heat".into(),
            Ok(other) => self.flash = other.into(),
            Err(e) => self.flash = e.to_string(),
        }
    }

    fn toggle_autonomous(&mut self, idx: usize) {
        match self.kitchen.toggle_autonomous(idx) {
            Ok(()) => {
                let a = &self.kitchen.agents[idx];
                self.flash = if a.autonomous {
                    "let him cook".into()
                } else {
                    "hands back on".into()
                };
            }
            Err(e) => self.flash = e.to_string(),
        }
    }

    fn do_spawn(&mut self, name: String, task: Option<String>) -> Result<()> {
        self.kitchen.spawn(&name, task)?;
        self.select_agent(self.kitchen.agents.len() - 1);
        self.flash = "on the line".into();
        Ok(())
    }

    fn refresh_stove_quiet(&mut self) {
        match self.refresh_stove() {
            Ok(()) => self.sync_stove_flash(),
            Err(e) => self.flash = e.to_string(),
        }
    }

    /// Drop stale "warming …" flash once burners have caught up.
    fn sync_stove_flash(&mut self) {
        if self.prompt.is_some() {
            return;
        }
        let any_warming = self.stove.iter().any(|s| s.heat() == Heat::Warming);
        if self.flash.starts_with("warming ") && !any_warming {
            self.flash.clear();
        }
    }

    fn handle(&mut self, key: KeyEvent) -> Result<bool> {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }
        if self.prompt.is_some() {
            return self.handle_prompt(key);
        }
        match self.screen {
            Screen::Kitchen => self.handle_kitchen(key),
            Screen::Inspect => self.handle_inspect(key),
            Screen::Recipes => self.handle_recipes(key),
            Screen::Pantry => self.handle_pantry(key),
            Screen::Stove => self.handle_stove(key),
        }
    }

    fn handle_prompt(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.prompt = None;
                self.input.clear();
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Enter => {
                let typed = self.input.trim().to_string();
                let kind = self.prompt.take();
                self.input.clear();
                match kind {
                    Some(Prompt::SpawnName) => {
                        if typed.is_empty() {
                            self.flash = "need a name".into();
                        } else {
                            self.prompt = Some(Prompt::SpawnTask { name: typed });
                        }
                    }
                    Some(Prompt::SpawnTask { name }) => {
                        let task = if typed.is_empty() { None } else { Some(typed) };
                        self.do_spawn(name, task)?;
                    }
                    Some(Prompt::AssignTask) => {
                        if typed.is_empty() {
                            self.flash = "need a ticket".into();
                        } else if let Some(i) = self.agent_idx() {
                            match self.kitchen.assign_task(i, &typed) {
                                Ok(()) => self.flash = "ticket in".into(),
                                Err(e) => self.flash = e.to_string(),
                            }
                        } else {
                            self.flash = "select an agent first".into();
                        }
                    }
                    Some(Prompt::RecipeName) => {
                        match self.kitchen.new_recipe(&typed, self.agent_idx()) {
                            Ok(()) => {
                                self.select_recipe(self.kitchen.recipes.len() - 1);
                                self.flash = "recipe filed".into();
                            }
                            Err(e) => self.flash = e.to_string(),
                        }
                    }
                    None => {}
                }
            }
            KeyCode::Char(c) => self.input.push(c),
            _ => {}
        }
        Ok(false)
    }

    fn handle_kitchen(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.agent_idx().unwrap_or(0).saturating_sub(1);
                self.select_agent(i);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.agent_idx().unwrap_or(0).saturating_add(1);
                self.select_agent(i);
            }
            KeyCode::Enter => {
                if self.agent_idx().is_some() {
                    self.screen = Screen::Inspect;
                } else {
                    self.flash = empty_kitchen_msg().into();
                }
            }
            KeyCode::Char('s') => {
                self.input.clear();
                self.prompt = Some(Prompt::SpawnName);
            }
            KeyCode::Char('t') => {
                if self.agent_idx().is_some() {
                    self.input.clear();
                    self.prompt = Some(Prompt::AssignTask);
                } else {
                    self.flash = "select an agent first".into();
                }
            }
            KeyCode::Char('r') => self.screen = Screen::Recipes,
            KeyCode::Char('p') => {
                self.refresh_pantry()?;
                self.screen = Screen::Pantry;
            }
            KeyCode::Char('o') => {
                self.refresh_stove()?;
                self.screen = Screen::Stove;
            }
            KeyCode::Char('m') => {
                if let Some(i) = self.agent_idx() {
                    self.toggle_autonomous(i);
                } else {
                    self.flash = "select an agent first".into();
                }
            }
            KeyCode::Char('c') => {
                if let Some(i) = self.agent_idx() {
                    if let Err(e) = self.kitchen.plate(i) {
                        self.flash = e.to_string();
                    } else {
                        self.flash = "plated".into();
                    }
                } else {
                    self.flash = "select an agent first".into();
                }
            }
            KeyCode::Char('x') => {
                if let Some(i) = self.agent_idx() {
                    if let Err(e) = self.kitchen.burn(i) {
                        self.flash = e.to_string();
                    } else {
                        self.flash = "burnt".into();
                    }
                } else {
                    self.flash = "select an agent first".into();
                }
            }
            KeyCode::Char('d') => {
                if let Some(i) = self.agent_idx() {
                    self.stop_agent(i);
                } else {
                    self.flash = "select an agent first".into();
                }
            }
            KeyCode::Delete | KeyCode::Char('D') => {
                if let Some(i) = self.agent_idx() {
                    if let Err(e) = self.remove_agent(i) {
                        self.flash = e.to_string();
                    }
                } else {
                    self.flash = "select an agent first".into();
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_inspect(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Esc | KeyCode::Char('b') => self.screen = Screen::Kitchen,
            KeyCode::Char('t') => {
                self.input.clear();
                self.prompt = Some(Prompt::AssignTask);
            }
            KeyCode::Char('m') => {
                if let Some(i) = self.agent_idx() {
                    self.toggle_autonomous(i);
                }
            }
            KeyCode::Char('r') => self.screen = Screen::Recipes,
            KeyCode::Char('p') => {
                self.refresh_pantry()?;
                self.screen = Screen::Pantry;
            }
            KeyCode::Char('o') => {
                self.refresh_stove()?;
                self.screen = Screen::Stove;
            }
            KeyCode::Char('c') => {
                if let Some(i) = self.agent_idx() {
                    if let Err(e) = self.kitchen.plate(i) {
                        self.flash = e.to_string();
                    } else {
                        self.flash = "plated".into();
                    }
                }
            }
            KeyCode::Char('x') => {
                if let Some(i) = self.agent_idx() {
                    if let Err(e) = self.kitchen.burn(i) {
                        self.flash = e.to_string();
                    } else {
                        self.flash = "burnt".into();
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(i) = self.agent_idx() {
                    self.stop_agent(i);
                }
            }
            KeyCode::Delete | KeyCode::Char('D') => {
                if let Some(i) = self.agent_idx() {
                    if let Err(e) = self.remove_agent(i) {
                        self.flash = e.to_string();
                    }
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_recipes(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Esc | KeyCode::Char('b') => self.screen = Screen::Kitchen,
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.recipe_idx().unwrap_or(0).saturating_sub(1);
                self.select_recipe(i);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.recipe_idx().unwrap_or(0).saturating_add(1);
                self.select_recipe(i);
            }
            KeyCode::Char('n') => {
                self.input.clear();
                self.prompt = Some(Prompt::RecipeName);
            }
            KeyCode::Char('a') => match (self.agent_idx(), self.recipe_idx()) {
                (Some(a), Some(r)) => {
                    self.kitchen.apply_recipe(a, r)?;
                    self.flash = "recipe on the agent".into();
                }
                _ => self.flash = "need an agent (kitchen) and a recipe".into(),
            },
            KeyCode::Char('D') => {
                if let Some(i) = self.recipe_idx() {
                    self.kitchen.delete_recipe(i)?;
                    if self.kitchen.recipes.is_empty() {
                        self.recipe_list.select(None);
                    } else {
                        self.select_recipe(i.min(self.kitchen.recipes.len() - 1));
                    }
                    self.flash = "recipe deleted".into();
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_pantry(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Esc | KeyCode::Char('b') => self.screen = Screen::Kitchen,
            KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                self.pantry_col = match self.pantry_col {
                    PantryCol::Skills => PantryCol::Rules,
                    PantryCol::Rules => PantryCol::Skills,
                };
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_pantry(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_pantry(1),
            KeyCode::Char('a') | KeyCode::Enter => self.pantry_attach()?,
            KeyCode::Char('-') => self.pantry_detach()?,
            _ => {}
        }
        Ok(false)
    }

    fn stove_simmer(&mut self) {
        match self.selected_burner() {
            Some(b) => match stove::simmer(b) {
                Ok(msg) => {
                    self.refresh_stove_quiet();
                    self.flash = msg;
                }
                Err(e) => {
                    self.refresh_stove_quiet();
                    self.flash = e.to_string();
                }
            },
            None => self.flash = "pick a burner".into(),
        }
    }

    fn handle_stove(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Esc | KeyCode::Char('b') => self.screen = Screen::Kitchen,
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.burner_idx().unwrap_or(0).saturating_sub(1);
                self.select_burner(i);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.burner_idx().unwrap_or(0).saturating_add(1);
                self.select_burner(i);
            }
            KeyCode::Enter | KeyCode::Char('s') => self.stove_simmer(),
            KeyCode::Char('c') => match self.selected_burner() {
                Some(b) => match stove::cool(b) {
                    Ok(msg) => {
                        self.refresh_stove_quiet();
                        self.flash = msg;
                    }
                    Err(e) => {
                        self.refresh_stove_quiet();
                        self.flash = e.to_string();
                    }
                },
                None => self.flash = "pick a burner".into(),
            },
            KeyCode::Char('r') => {
                self.refresh_stove_quiet();
                self.flash = "stove checked".into();
            }
            _ => {}
        }
        Ok(false)
    }

    fn pantry_attach(&mut self) -> Result<()> {
        let Some(ai) = self.agent_idx() else {
            self.flash = "select an agent in the kitchen first".into();
            return Ok(());
        };
        match self.pantry_col {
            PantryCol::Skills => {
                let Some(i) = self.skill_list.selected() else {
                    self.flash = "pick a skill".into();
                    return Ok(());
                };
                let Some(name) = self.skills.get(i).cloned() else {
                    self.flash = "no skills in pantry".into();
                    return Ok(());
                };
                self.kitchen.add_skill(ai, &name)?;
                self.flash = format!("+ skill {name}");
            }
            PantryCol::Rules => {
                let Some(i) = self.rule_list.selected() else {
                    self.flash = "pick a rule".into();
                    return Ok(());
                };
                let Some(name) = self.rules.get(i).cloned() else {
                    self.flash = "no rules in pantry".into();
                    return Ok(());
                };
                self.kitchen.add_rule(ai, &name)?;
                self.flash = format!("+ rule {name}");
            }
        }
        Ok(())
    }

    fn pantry_detach(&mut self) -> Result<()> {
        let Some(ai) = self.agent_idx() else {
            self.flash = "select an agent in the kitchen first".into();
            return Ok(());
        };
        let agent = self.kitchen.agents.get(ai).context("no agent")?;
        match self.pantry_col {
            PantryCol::Skills => {
                let Some(i) = self.skill_list.selected() else {
                    self.flash = "pick a skill".into();
                    return Ok(());
                };
                let Some(name) = self.skills.get(i).cloned() else {
                    return Ok(());
                };
                if !agent.skills.iter().any(|s| s == &name) {
                    self.flash = format!("{name} not on agent");
                    return Ok(());
                }
                self.kitchen.remove_skill(ai, &name)?;
                self.flash = format!("- skill {name}");
            }
            PantryCol::Rules => {
                let Some(i) = self.rule_list.selected() else {
                    self.flash = "pick a rule".into();
                    return Ok(());
                };
                let Some(name) = self.rules.get(i).cloned() else {
                    return Ok(());
                };
                if !agent.rules.iter().any(|s| s == &name) {
                    self.flash = format!("{name} not on agent");
                    return Ok(());
                };
                self.kitchen.remove_rule(ai, &name)?;
                self.flash = format!("- rule {name}");
            }
        }
        Ok(())
    }

    fn move_pantry(&mut self, delta: i32) {
        let (len, state) = match self.pantry_col {
            PantryCol::Skills => (self.skills.len(), &mut self.skill_list),
            PantryCol::Rules => (self.rules.len(), &mut self.rule_list),
        };
        if len == 0 {
            return;
        }
        let cur = state.selected().unwrap_or(0) as i32;
        let next = (cur + delta).clamp(0, len as i32 - 1) as usize;
        state.select(Some(next));
    }
}

pub fn run() -> Result<()> {
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal);
    ratatui::restore();
    result
}

fn run_app(terminal: &mut DefaultTerminal) -> Result<()> {
    let mut app = App::new()?;
    loop {
        if stove_poll_needed(&app) {
            app.refresh_stove_quiet();
        }
        terminal.draw(|frame| draw(frame, &mut app))?;
        if !event::poll(poll_interval(&app))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if app.handle(key)? {
                break;
            }
        }
    }
    Ok(())
}

fn stove_poll_needed(app: &App) -> bool {
    app.screen == Screen::Stove
        || app.stove.iter().any(|s| s.heat() == Heat::Warming)
        || app.flash.starts_with("warming ")
}

fn poll_interval(app: &App) -> Duration {
    if app.stove.iter().any(|s| s.heat() == Heat::Warming)
        || app.flash.starts_with("warming ")
    {
        Duration::from_millis(250)
    } else if app.screen == Screen::Stove {
        Duration::from_millis(750)
    } else {
        Duration::from_secs(60 * 60)
    }
}

fn draw(frame: &mut Frame, app: &mut App) {
    let inner = Layout::vertical([Constraint::Percentage(100)])
        .margin(2)
        .split(frame.area())[0];

    let status_h = 3;
    let chunks = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(status_h),
        Constraint::Length(1),
    ])
    .split(inner);

    let footer_h = status_h + 1;
    let main_area = chunks[0];
    let main_h = match app.screen {
        Screen::Kitchen => kitchen_panel_height(app).min(main_area.height),
        _ => main_area
            .height
            .saturating_sub(footer_h)
            .min(main_area.height * 2 / 3),
    };
    let panel = Rect {
        x: main_area.x,
        y: main_area.y,
        width: main_area.width,
        height: main_h,
    };

    match app.screen {
        Screen::Kitchen => draw_kitchen(frame, app, panel),
        Screen::Inspect => draw_inspect(frame, app, panel),
        Screen::Recipes => draw_recipes(frame, app, panel),
        Screen::Pantry => draw_pantry(frame, app, panel),
        Screen::Stove => draw_stove(frame, app, panel),
    }

    draw_status_bar(frame, app, chunks[1]);
    frame.render_widget(
        help_line(app).style(Style::default().fg(theme::muted())),
        chunks[2],
    );
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::muted()))
        .title(status_bar_title(app))
        .title_style(theme::ink_bold());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let style = if app.prompt.is_some() {
        theme::ink()
    } else if !app.flash.is_empty() {
        Style::default().fg(theme::heat())
    } else {
        theme::ink()
    };
    frame.render_widget(Paragraph::new(status_message(app)).style(style), inner);
}

fn status_bar_title(app: &App) -> &'static str {
    if let Some(prompt) = &app.prompt {
        match prompt {
            Prompt::SpawnName => "Name",
            Prompt::SpawnTask { .. } => "Ticket",
            Prompt::AssignTask => "Ticket",
            Prompt::RecipeName => "Recipe",
        }
    } else {
        "Status"
    }
}

fn status_message(app: &App) -> Line<'static> {
    if app.prompt.is_some() {
        return Line::from(app.input.clone());
    }
    if !app.flash.is_empty() {
        return Line::from(truncate(&app.flash, 120).to_string());
    }
    idle_status(app)
}

fn idle_status(app: &App) -> Line<'static> {
    match app.screen {
        Screen::Kitchen => Line::from(app.kitchen.line_summary()),
        Screen::Inspect => {
            if let Some(i) = app.agent_idx() {
                if let Some(a) = app.kitchen.agents.get(i) {
                    let task = a.task.as_deref().unwrap_or("Idle");
                    return Line::from(format!("{} — {}", a.name, task));
                }
            }
            Line::from("inspect")
        }
        Screen::Recipes => Line::from(format!("{} recipes", app.kitchen.recipes.len())),
        Screen::Pantry => Line::from(format!(
            "{} skills · {} rules",
            app.skills.len(),
            app.rules.len()
        )),
        Screen::Stove => {
            let hot = app
                .stove
                .iter()
                .filter(|s| s.heat() != Heat::Cold)
                .count();
            Line::from(format!("{}/{} burners hot", hot, app.stove.len()))
        }
    }
}

/// Kitchen panel height from roster size — avoids a full-terminal empty box.
fn kitchen_panel_height(app: &App) -> u16 {
    let agents = app.kitchen.agents.len();
    let list_rows = agents.max(1) as u16;
    let actions = if app.agent_idx().is_some() && agents > 0 {
        1
    } else {
        0
    };
    // borders + summary + list + optional action row + stats (rule + counts)
    2 + 1 + list_rows + actions + 2
}

fn empty_kitchen_msg() -> &'static str {
    let tick = chrono::Local::now().timestamp().unsigned_abs() as usize / 6;
    EMPTY_KITCHEN[tick % EMPTY_KITCHEN.len()]
}

fn help_line(app: &App) -> Line<'static> {
    if app.prompt.is_some() {
        return Line::from("[ENTER]:Confirm  [ESC]:Cancel");
    }
    match app.screen {
        Screen::Kitchen => kitchen_help(app),
        Screen::Inspect => Line::from(
            "[ESC]:Kitchen  [M]:Autonomous  [C]:Plate  [D]:Stop  [DEL]:Dismiss  [T]:Ticket  [Q]:Quit",
        ),
        Screen::Recipes => Line::from("[ESC]:Kitchen  [N]:New  [A]:Apply  [DEL]:Delete  [Q]:Quit"),
        Screen::Pantry => {
            Line::from("[ESC]:Kitchen  [TAB]:Column  [A]:Attach  [-]:Detach  [Q]:Quit")
        }
        Screen::Stove => {
            Line::from("[ESC]:Kitchen  [ENTER]:Simmer  [C]:Cool  [R]:Refresh  [Q]:Quit")
        }
    }
}

fn kitchen_help(app: &App) -> Line<'static> {
    if app.agent_idx().is_some() {
        Line::from(
            "[J/K]:Move  [ENTER]:Inspect  [T]:Ticket  [P]:Pantry  [O]:Stove  [D]:Stop  [DEL]:Dismiss  [X]:Burnt  [R]:Recipes  [Q]:Quit",
        )
    } else {
        Line::from("[S]:Spawn  [R]:Recipes  [P]:Pantry  [O]:Stove  [Q]:Quit")
    }
}

fn selected_action_row(app: &App) -> Option<Line<'static>> {
    let i = app.agent_idx()?;
    let a = app.kitchen.agents.get(i)?;
    let name = a.name.clone();
    let cook_label = if a.autonomous {
        Span::styled("[m] hands back on", Style::default().fg(theme::muted()))
    } else {
        Span::styled(
            "[m] LET HIM COOK",
            Style::default()
                .fg(theme::heat())
                .add_modifier(Modifier::BOLD),
        )
    };
    Some(Line::from(vec![
        Span::styled("→ ", Style::default().fg(theme::muted())),
        Span::styled(name, theme::ink_bold()),
        Span::raw("   "),
        Span::styled("🍳 ", Style::default().fg(theme::heat())),
        cook_label,
        Span::raw("   "),
        Span::styled(
            "[c] plate",
            Style::default()
                .fg(theme::sage())
                .add_modifier(Modifier::BOLD),
        ),
    ]))
}

fn draw_kitchen(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::muted()))
        .title(" KITCHEN ")
        .title_style(theme::ink_bold());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let show_actions = app.agent_idx().is_some();
    let body = if show_actions {
        Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .split(inner)
    } else {
        Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(inner)
    };

    frame.render_widget(
        Paragraph::new(app.kitchen.line_summary()).style(
            Style::default()
                .fg(theme::muted())
                .add_modifier(Modifier::ITALIC),
        ),
        body[0],
    );

    let list_idx = 1usize;
    let stats_idx = if show_actions { 3 } else { 2 };
    let action_idx = if show_actions { Some(2) } else { None };

    let items: Vec<ListItem> = if app.kitchen.agents.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            empty_kitchen_msg(),
            Style::default()
                .fg(theme::muted())
                .add_modifier(Modifier::ITALIC),
        )))]
    } else {
        app.kitchen
            .agents
            .iter()
            .map(|a| ListItem::new(agent_line(a)))
            .collect()
    };

    let list = List::new(items)
        .highlight_style(theme::highlight())
        .highlight_symbol("");
    frame.render_stateful_widget(list, body[list_idx], &mut app.agent_list);

    if let Some(idx) = action_idx {
        if let Some(row) = selected_action_row(app) {
            frame.render_widget(Paragraph::new(row), body[idx]);
        }
    }

    let (sk, ru) = (app.skills.len(), app.rules.len());
    let rec = app.kitchen.recipes.len();
    let stats = format!("Skills: {sk:<4}  Rules: {ru:<4}  Recipes: {rec}");
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "─".repeat(body[stats_idx].width as usize),
                Style::default().fg(theme::muted()),
            )),
            Line::from(Span::styled(stats, Style::default().fg(theme::muted()))),
        ]),
        body[stats_idx],
    );
}

fn agent_line(a: &kitchen::kitchen::Agent) -> Line<'static> {
    let hot = a.status == Status::Cooking;
    let status_style = match a.status {
        Status::Cooking => Style::default().fg(theme::heat()).add_modifier(if hot {
            Modifier::BOLD
        } else {
            Modifier::empty()
        }),
        Status::Cooked => Style::default().fg(theme::sage()),
        Status::Burnt => Style::default().fg(theme::charred()),
        Status::Raw => Style::default().fg(theme::muted()),
    };
    let name_style = if hot { theme::ink_bold() } else { theme::ink() };
    let task = a.task.as_deref().unwrap_or("Idle").to_string();
    let label = format!("{:<14}", a.status.label(a.autonomous));
    let recipe = a.recipe.as_deref().unwrap_or("—");
    let recipe = truncate(recipe, 16);
    Line::from(vec![
        Span::styled(format!("{} ", a.status.bullet()), status_style),
        Span::styled(format!("{:<16}", a.name), name_style),
        Span::styled(
            format!("{} {}  ", a.status.glyph(a.autonomous), label),
            status_style,
        ),
        Span::styled(
            format!("{:<16}  ", recipe),
            Style::default().fg(theme::muted()),
        ),
        Span::styled(
            task,
            if hot {
                theme::ink()
            } else {
                Style::default().fg(theme::muted())
            },
        ),
    ])
}

fn draw_inspect(frame: &mut Frame, app: &App, area: Rect) {
    let Some(i) = app.agent_idx() else {
        return;
    };
    let Some(a) = app.kitchen.agents.get(i) else {
        return;
    };
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(area);

    let action = if a.autonomous {
        Line::from(vec![
            Span::styled(
                "🍳 LET HIM COOK",
                Style::default()
                    .fg(theme::heat())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" — hands off    "),
            Span::styled("[m] take back", Style::default().fg(theme::muted())),
            Span::raw("    "),
            Span::styled(
                "[c] plate",
                Style::default()
                    .fg(theme::sage())
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                "[m] LET HIM COOK",
                Style::default()
                    .fg(theme::heat())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("    "),
            Span::styled(
                "[c] plate",
                Style::default()
                    .fg(theme::sage())
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    };

    let recipe = a.recipe.as_deref().unwrap_or("—");
    let skills = if a.skills.is_empty() {
        "—".into()
    } else {
        a.skills.join(", ")
    };
    let rules = if a.rules.is_empty() {
        "—".into()
    } else {
        a.rules.join(", ")
    };
    let task = a.task.as_deref().unwrap_or("Idle");
    let meta = vec![
        Line::from(Span::styled(a.name.clone(), theme::ink_bold())),
        Line::from(""),
        action,
        Line::from(""),
        kv("status", a.status.label(a.autonomous)),
        kv("task", task),
        kv("recipe", recipe),
        kv("skills", &skills),
        kv("rules", &rules),
        Line::from(""),
        Line::from(Span::styled(
            "status is local until a runtime is wired",
            Style::default()
                .fg(theme::muted())
                .add_modifier(Modifier::ITALIC),
        )),
    ];
    frame.render_widget(
        Paragraph::new(meta).wrap(Wrap { trim: true }).block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::muted()))
                .title(" STATION "),
        ),
        cols[0],
    );

    let log = read_log(&a.id);
    let log = if log.trim().is_empty() {
        "no tickets yet.".into()
    } else {
        log
    };
    frame.render_widget(
        Paragraph::new(log)
            .wrap(Wrap { trim: false })
            .style(theme::ink())
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme::muted()))
                    .title(" TICKETS / LOG "),
            ),
        cols[1],
    );
}

fn kv<'a>(k: &'a str, v: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{k:<14}"), Style::default().fg(theme::muted())),
        Span::styled(v.to_string(), theme::ink()),
    ])
}

fn kv_owned(k: &str, v: String) -> Line<'static> {
    kv_styled(k, v, theme::ink())
}

fn kv_styled(k: &str, v: String, v_style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{k:<14}"), Style::default().fg(theme::muted())),
        Span::styled(v, v_style),
    ])
}

fn draw_recipes(frame: &mut Frame, app: &mut App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);

    let items: Vec<ListItem> = if app.kitchen.recipes.is_empty() {
        vec![ListItem::new(Span::styled(
            "no recipes. [n] file one.",
            Style::default()
                .fg(theme::muted())
                .add_modifier(Modifier::ITALIC),
        ))]
    } else {
        app.kitchen
            .recipes
            .iter()
            .map(|r| {
                ListItem::new(format!(
                    "{}   {}s {}r",
                    r.name,
                    r.skills.len(),
                    r.rules.len()
                ))
            })
            .collect()
    };
    let list = List::new(items)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::muted()))
                .title(" RECIPES "),
        )
        .highlight_style(theme::highlight());
    frame.render_stateful_widget(list, cols[0], &mut app.recipe_list);

    let detail = if let Some(r) = app.recipe_idx().and_then(|i| app.kitchen.recipes.get(i)) {
        let skills = if r.skills.is_empty() {
            "—".into()
        } else {
            r.skills.join("\n")
        };
        let rules = if r.rules.is_empty() {
            "—".into()
        } else {
            r.rules.join("\n")
        };
        vec![
            Line::from(Span::styled(r.name.clone(), theme::ink_bold())),
            Line::from(""),
            Line::from(Span::styled("skills", Style::default().fg(theme::muted()))),
            Line::from(Span::styled(skills, theme::ink())),
            Line::from(""),
            Line::from(Span::styled("rules", Style::default().fg(theme::muted()))),
            Line::from(Span::styled(rules, theme::ink())),
        ]
    } else {
        vec![Line::from(Span::styled(
            "a recipe is a reusable skill + rule plate.",
            Style::default().fg(theme::muted()),
        ))]
    };
    frame.render_widget(
        Paragraph::new(detail).wrap(Wrap { trim: true }).block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::muted()))
                .title(" PLATE "),
        ),
        cols[1],
    );
}

fn draw_pantry(frame: &mut Frame, app: &mut App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let skill_items: Vec<ListItem> = if app.skills.is_empty() {
        vec![ListItem::new(Span::styled(
            "no skills in skills/",
            Style::default()
                .fg(theme::muted())
                .add_modifier(Modifier::ITALIC),
        ))]
    } else {
        app.skills
            .iter()
            .map(|s| ListItem::new(s.clone()))
            .collect()
    };
    let rule_items: Vec<ListItem> = if app.rules.is_empty() {
        vec![ListItem::new(Span::styled(
            "no rules in rules/",
            Style::default()
                .fg(theme::muted())
                .add_modifier(Modifier::ITALIC),
        ))]
    } else {
        app.rules.iter().map(|s| ListItem::new(s.clone())).collect()
    };

    let skill_border = if matches!(app.pantry_col, PantryCol::Skills) {
        theme::heat()
    } else {
        theme::muted()
    };
    let rule_border = if matches!(app.pantry_col, PantryCol::Rules) {
        theme::heat()
    } else {
        theme::muted()
    };

    let skills = List::new(skill_items)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(skill_border))
                .title(" SKILLS "),
        )
        .highlight_style(theme::highlight());
    let rules = List::new(rule_items)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(rule_border))
                .title(" RULES "),
        )
        .highlight_style(theme::highlight());

    frame.render_stateful_widget(skills, cols[0], &mut app.skill_list);
    frame.render_stateful_widget(rules, cols[1], &mut app.rule_list);
}

fn draw_stove(frame: &mut Frame, app: &mut App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    let items: Vec<ListItem> = app
        .stove
        .iter()
        .map(|s| ListItem::new(burner_line(s)))
        .collect();
    let list = List::new(items)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::muted()))
                .title(" STOVE "),
        )
        .highlight_style(theme::highlight());
    frame.render_stateful_widget(list, cols[0], &mut app.burner_list);

    let detail = if let Some(st) = app.burner_idx().and_then(|i| app.stove.get(i)) {
        burner_detail(st)
    } else {
        vec![Line::from(Span::styled(
            "local model servers — MLX and Ollama.",
            Style::default().fg(theme::muted()),
        ))]
    };
    frame.render_widget(
        Paragraph::new(detail).wrap(Wrap { trim: true }).block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::muted()))
                .title(" BURNER "),
        ),
        cols[1],
    );
}

fn burner_line(st: &BurnerStatus) -> Line<'static> {
    let heat = st.heat();
    let heat_style = match heat {
        Heat::Simmering => Style::default().fg(theme::heat()),
        Heat::Warming => Style::default()
            .fg(theme::heat())
            .add_modifier(Modifier::ITALIC),
        Heat::Cold => Style::default().fg(theme::muted()),
    };
    Line::from(vec![
        Span::styled(format!("{:<8}", st.burner.label()), theme::ink_bold()),
        Span::raw("  "),
        Span::styled(format!("{:<10}", heat.label()), heat_style),
        Span::styled(
            format!(":{}", st.port),
            Style::default().fg(theme::muted()),
        ),
    ])
}

fn burner_detail(st: &BurnerStatus) -> Vec<Line<'static>> {
    let heat = st.heat();
    let heat_style = match heat {
        Heat::Simmering => Style::default().fg(theme::heat()),
        Heat::Warming => Style::default()
            .fg(theme::heat())
            .add_modifier(Modifier::ITALIC),
        Heat::Cold => Style::default().fg(theme::muted()),
    };
    let pid = st.pid.map(|p| p.to_string()).unwrap_or_else(|| "—".into());
    let owner = match heat {
        Heat::Cold => "—".to_string(),
        _ if st.managed => "kitchen".to_string(),
        _ => "external".to_string(),
    };
    let url = format!("127.0.0.1:{}", st.port);
    let log = ellipsize_path(&stove::log_path(st.burner), 40);
    vec![
        Line::from(Span::styled(st.burner.label(), theme::ink_bold())),
        Line::from(""),
        kv_styled("heat", heat.label().to_string(), heat_style),
        kv_owned("port", st.port.to_string()),
        kv_owned("pid", pid),
        kv_owned("started by", owner),
        kv_owned("url", url),
        kv_styled("log", log, Style::default().fg(theme::muted())),
    ]
}

fn truncate(s: &str, max: usize) -> &str {
    if s.chars().count() <= max {
        s
    } else {
        &s[..s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(s.len())]
    }
}

fn ellipsize_path(path: &std::path::Path, max: usize) -> String {
    let raw = shorten_home(path);
    if raw.chars().count() <= max {
        return raw;
    }
    let keep = max.saturating_sub(1);
    format!("{}…", truncate(&raw, keep))
}

fn shorten_home(path: &std::path::Path) -> String {
    let s = path.display().to_string();
    if let Ok(home) = std::env::var("HOME") {
        if let Some(rest) = s.strip_prefix(&home) {
            return format!("~{rest}");
        }
    }
    s
}
