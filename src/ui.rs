//! Cook kitchen TUI. Cooking words are labels; navigation stays boring and fast.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};

use cook::kitchen::{read_log, Kitchen, Status};

const EMPTY_KITCHEN: [&str; 6] = [
    "stove's cold.  [s] spawn someone.",
    "nothing cooking.  [s] spawn someone.",
    "kitchen's empty.",
    "no one's cooking.",
    "we got no cooks.",
    "someone get in here.  [s] spawn someone.",
];

const HEAT: Color = Color::Rgb(232, 140, 48);
const BONE: Color = Color::Rgb(232, 226, 216);
const SAGE: Color = Color::Rgb(138, 168, 122);
const CHAR: Color = Color::Rgb(196, 72, 48);
const ASH: Color = Color::Rgb(120, 118, 112);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Kitchen,
    Inspect,
    Recipes,
    Pantry,
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
        Ok(Self {
            kitchen,
            screen: Screen::Kitchen,
            agent_list,
            recipe_list,
            skill_list,
            rule_list,
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

    fn refresh_pantry(&mut self) -> Result<()> {
        let (skills, rules) = Kitchen::pantry()?;
        self.skills = skills;
        self.rules = rules;
        Ok(())
    }

    fn do_spawn(&mut self, name: String, task: Option<String>) -> Result<()> {
        self.kitchen.spawn(&name, task)?;
        self.select_agent(self.kitchen.agents.len() - 1);
        self.flash = "on the line".into();
        Ok(())
    }

    fn handle(&mut self, key: KeyEvent) -> Result<bool> {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }
        if self.prompt.is_some() {
            return self.handle_prompt(key);
        }
        self.flash.clear();
        match self.screen {
            Screen::Kitchen => self.handle_kitchen(key),
            Screen::Inspect => self.handle_inspect(key),
            Screen::Recipes => self.handle_recipes(key),
            Screen::Pantry => self.handle_pantry(key),
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
                        if let Some(i) = self.agent_idx() {
                            match self.kitchen.assign_task(i, &typed) {
                                Ok(()) => self.flash = "ticket in".into(),
                                Err(e) => self.flash = e.to_string(),
                            }
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
                }
            }
            KeyCode::Char('r') => self.screen = Screen::Recipes,
            KeyCode::Char('p') => {
                self.refresh_pantry()?;
                self.screen = Screen::Pantry;
            }
            KeyCode::Char('m') => {
                if let Some(i) = self.agent_idx() {
                    match self.kitchen.toggle_autonomous(i) {
                        Ok(()) => {
                            let a = &self.kitchen.agents[i];
                            self.flash = if a.autonomous {
                                "let him cook".into()
                            } else {
                                "hands back on".into()
                            };
                        }
                        Err(e) => self.flash = e.to_string(),
                    }
                }
            }
            KeyCode::Char('c') => {
                if let Some(i) = self.agent_idx() {
                    self.kitchen.plate(i)?;
                    self.flash = "plated".into();
                }
            }
            KeyCode::Char('x') => {
                if let Some(i) = self.agent_idx() {
                    self.kitchen.burn(i)?;
                    self.flash = "burnt".into();
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                if let Some(i) = self.agent_idx() {
                    match self.kitchen.kill(i) {
                        Ok("stopped") => self.flash = "off the heat".into(),
                        Ok(other) => self.flash = other.into(),
                        Err(e) => self.flash = e.to_string(),
                    }
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
                    if let Err(e) = self.kitchen.toggle_autonomous(i) {
                        self.flash = e.to_string();
                    }
                }
            }
            KeyCode::Char('r') => self.screen = Screen::Recipes,
            KeyCode::Char('p') => {
                self.refresh_pantry()?;
                self.screen = Screen::Pantry;
            }
            KeyCode::Char('c') => {
                if let Some(i) = self.agent_idx() {
                    self.kitchen.plate(i)?;
                }
            }
            KeyCode::Char('x') => {
                if let Some(i) = self.agent_idx() {
                    self.kitchen.burn(i)?;
                }
            }
            KeyCode::Char('d') => {
                if let Some(i) = self.agent_idx() {
                    let _ = self.kitchen.kill(i)?;
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
                    self.select_recipe(i.saturating_sub(0));
                    if self.kitchen.recipes.is_empty() {
                        self.recipe_list.select(None);
                    } else {
                        self.select_recipe(i.min(self.kitchen.recipes.len() - 1));
                    }
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
            KeyCode::Char('a') | KeyCode::Enter => {
                let Some(ai) = self.agent_idx() else {
                    self.flash = "spawn or select an agent in the kitchen first".into();
                    return Ok(false);
                };
                match self.pantry_col {
                    PantryCol::Skills => {
                        if let Some(i) = self.skill_list.selected() {
                            if let Some(name) = self.skills.get(i).cloned() {
                                self.kitchen.add_skill(ai, &name)?;
                                self.flash = format!("+ skill {name}");
                            }
                        }
                    }
                    PantryCol::Rules => {
                        if let Some(i) = self.rule_list.selected() {
                            if let Some(name) = self.rules.get(i).cloned() {
                                self.kitchen.add_rule(ai, &name)?;
                                self.flash = format!("+ rule {name}");
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(false)
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
        terminal.draw(|frame| draw(frame, &mut app))?;
        if let Event::Key(key) = event::read()? {
            if app.handle(key)? {
                break;
            }
        }
    }
    Ok(())
}

fn draw(frame: &mut Frame, app: &mut App) {
    let bone = Style::default().fg(BONE);
    let chunks = Layout::vertical([
        Constraint::Min(8),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(frame.area());

    match app.screen {
        Screen::Kitchen => draw_kitchen(frame, app, chunks[0]),
        Screen::Inspect => draw_inspect(frame, app, chunks[0]),
        Screen::Recipes => draw_recipes(frame, app, chunks[0]),
        Screen::Pantry => draw_pantry(frame, app, chunks[0]),
    }

    if let Some(prompt) = &app.prompt {
        let title = match prompt {
            Prompt::SpawnName => "name",
            Prompt::SpawnTask { .. } => "ticket (empty = idle)",
            Prompt::AssignTask => "ticket",
            Prompt::RecipeName => "recipe name (copies selected agent pantry)",
        };
        frame.render_widget(
            Paragraph::new(format!("{title}: {}", app.input)).style(bone.fg(HEAT)),
            chunks[1],
        );
    } else {
        frame.render_widget(
            Paragraph::new(truncate(&app.flash, 120)).style(Style::default().fg(ASH)),
            chunks[1],
        );
    }

    frame.render_widget(help_line(app).style(Style::default().fg(ASH)), chunks[2]);
}

fn empty_kitchen_msg() -> &'static str {
    let tick = chrono::Local::now().timestamp().unsigned_abs() as usize / 6;
    EMPTY_KITCHEN[tick % EMPTY_KITCHEN.len()]
}

fn help_line(app: &App) -> Paragraph<'static> {
    if app.prompt.is_some() {
        return Paragraph::new("enter confirm   esc cancel");
    }
    match app.screen {
        Screen::Kitchen => kitchen_help(app),
        Screen::Inspect => Paragraph::new(
            "[esc] kitchen  [m] let him cook  [c] plate  [t] ticket  [d] kill  [q] quit",
        ),
        Screen::Recipes => {
            Paragraph::new("[esc] kitchen  [n] new  [a] apply  [D] delete  [q] quit")
        }
        Screen::Pantry => Paragraph::new("[esc] kitchen  tab column  [a] attach  [q] quit"),
    }
}

fn kitchen_help(app: &App) -> Paragraph<'static> {
    if app.agent_idx().is_some() {
        Paragraph::new(Line::from(vec![Span::raw(
            "j/k  [enter] inspect  [t] ticket  [d] kill  [x] burnt  [r] recipes  [q] quit",
        )]))
    } else {
        Paragraph::new("[s] spawn  [r] recipes  [p] pantry  [q] quit")
    }
}

fn selected_action_row(app: &App) -> Option<Line<'static>> {
    let i = app.agent_idx()?;
    let a = app.kitchen.agents.get(i)?;
    let name = a.name.clone();
    let cook_label = if a.autonomous {
        Span::styled("[m] hands back on", Style::default().fg(ASH))
    } else {
        Span::styled(
            "[m] LET HIM COOK",
            Style::default().fg(HEAT).add_modifier(Modifier::BOLD),
        )
    };
    Some(Line::from(vec![
        Span::styled("→ ", Style::default().fg(ASH)),
        Span::styled(name, Style::default().fg(BONE).add_modifier(Modifier::BOLD)),
        Span::raw("   "),
        Span::styled("🍳 ", Style::default().fg(HEAT)),
        cook_label,
        Span::raw("   "),
        Span::styled(
            "[c] plate",
            Style::default().fg(SAGE).add_modifier(Modifier::BOLD),
        ),
    ]))
}

fn draw_kitchen(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ASH))
        .title(" KITCHEN ")
        .title_style(Style::default().fg(BONE).add_modifier(Modifier::BOLD));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let show_actions = app.agent_idx().is_some();
    let body = if show_actions {
        Layout::vertical([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .split(inner)
    } else {
        Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).split(inner)
    };

    let items: Vec<ListItem> = if app.kitchen.agents.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            empty_kitchen_msg(),
            Style::default().fg(ASH).add_modifier(Modifier::ITALIC),
        )))]
    } else {
        app.kitchen
            .agents
            .iter()
            .map(|a| ListItem::new(agent_line(a)))
            .collect()
    };

    let list = List::new(items)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED).fg(BONE))
        .highlight_symbol("");
    frame.render_stateful_widget(list, body[0], &mut app.agent_list);

    let stats_idx = if show_actions { 2 } else { 1 };
    if show_actions {
        if let Some(row) = selected_action_row(app) {
            frame.render_widget(Paragraph::new(row), body[1]);
        }
    }

    let (sk, ru) = (app.skills.len(), app.rules.len());
    let rec = app.kitchen.recipes.len();
    let stats = format!("Skills: {sk:<4}  Rules: {ru:<4}  Recipes: {rec}");
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "─".repeat(body[stats_idx].width as usize),
                Style::default().fg(ASH),
            )),
            Line::from(Span::styled(stats, Style::default().fg(ASH))),
        ]),
        body[stats_idx],
    );
}

fn agent_line(a: &cook::kitchen::Agent) -> Line<'static> {
    let hot = a.status == Status::Cooking;
    let status_style = match a.status {
        Status::Cooking => Style::default().fg(HEAT).add_modifier(if hot {
            Modifier::BOLD
        } else {
            Modifier::empty()
        }),
        Status::Cooked => Style::default().fg(SAGE),
        Status::Burnt => Style::default().fg(CHAR),
        Status::Raw => Style::default().fg(ASH),
    };
    let name_style = if hot {
        Style::default().fg(BONE).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(BONE)
    };
    let task = a.task.as_deref().unwrap_or("Idle").to_string();
    let label = format!("{:<14}", a.status.label(a.autonomous));
    Line::from(vec![
        Span::styled(format!("{} ", a.status.bullet()), status_style),
        Span::styled(format!("{:<16}", a.name), name_style),
        Span::styled(
            format!("{} {}  ", a.status.glyph(a.autonomous), label),
            status_style,
        ),
        Span::styled(task, Style::default().fg(if hot { BONE } else { ASH })),
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
                Style::default().fg(HEAT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" — hands off    "),
            Span::styled("[m] take back", Style::default().fg(ASH)),
            Span::raw("    "),
            Span::styled(
                "[c] plate",
                Style::default().fg(SAGE).add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                "[m] LET HIM COOK",
                Style::default().fg(HEAT).add_modifier(Modifier::BOLD),
            ),
            Span::raw("    "),
            Span::styled(
                "[c] plate",
                Style::default().fg(SAGE).add_modifier(Modifier::BOLD),
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
        Line::from(Span::styled(
            a.name.clone(),
            Style::default().fg(BONE).add_modifier(Modifier::BOLD),
        )),
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
            Style::default().fg(ASH).add_modifier(Modifier::ITALIC),
        )),
    ];
    frame.render_widget(
        Paragraph::new(meta).wrap(Wrap { trim: true }).block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(ASH))
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
            .style(Style::default().fg(BONE))
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(ASH))
                    .title(" TICKETS / LOG "),
            ),
        cols[1],
    );
}

fn kv<'a>(k: &'a str, v: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{k:<14}"), Style::default().fg(ASH)),
        Span::styled(v.to_string(), Style::default().fg(BONE)),
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
            Style::default().fg(ASH).add_modifier(Modifier::ITALIC),
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
                .border_style(Style::default().fg(ASH))
                .title(" RECIPES "),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED).fg(BONE));
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
            Line::from(Span::styled(
                r.name.clone(),
                Style::default().fg(BONE).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled("skills", Style::default().fg(ASH))),
            Line::from(Span::styled(skills, Style::default().fg(BONE))),
            Line::from(""),
            Line::from(Span::styled("rules", Style::default().fg(ASH))),
            Line::from(Span::styled(rules, Style::default().fg(BONE))),
        ]
    } else {
        vec![Line::from(Span::styled(
            "a recipe is a reusable skill + rule plate.",
            Style::default().fg(ASH),
        ))]
    };
    frame.render_widget(
        Paragraph::new(detail).wrap(Wrap { trim: true }).block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(ASH))
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
            Style::default().fg(ASH).add_modifier(Modifier::ITALIC),
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
            Style::default().fg(ASH).add_modifier(Modifier::ITALIC),
        ))]
    } else {
        app.rules.iter().map(|s| ListItem::new(s.clone())).collect()
    };

    let skill_border = if matches!(app.pantry_col, PantryCol::Skills) {
        HEAT
    } else {
        ASH
    };
    let rule_border = if matches!(app.pantry_col, PantryCol::Rules) {
        HEAT
    } else {
        ASH
    };

    let skills = List::new(skill_items)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(skill_border))
                .title(" SKILLS "),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED).fg(BONE));
    let rules = List::new(rule_items)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(rule_border))
                .title(" RULES "),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED).fg(BONE));

    frame.render_stateful_widget(skills, cols[0], &mut app.skill_list);
    frame.render_stateful_widget(rules, cols[1], &mut app.rule_list);
}

fn truncate(s: &str, max: usize) -> &str {
    if s.chars().count() <= max {
        s
    } else {
        &s[..s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(s.len())]
    }
}
