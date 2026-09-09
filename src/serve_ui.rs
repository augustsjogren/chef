//! Harness symlink UI. Same pantry, different door — link, don't browse.

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, ListState, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use slop_chef::config::{expand_path, Harness};
use slop_chef::item::{classify, items_for, Item, LinkState};
use slop_chef::ops::{link_item, unlink_item};
use slop_chef::{load_harnesses, pantry_root, tool_root};

use crate::theme;

#[derive(Clone, Copy)]
enum Screen {
    Harnesses,
    Items,
    ProjectPrompt,
}

struct Row {
    item: Item,
    state: LinkState,
}

struct App {
    harnesses: Vec<Harness>,
    harness_idx: usize,
    harness_list: ListState,
    items: Vec<Row>,
    item_idx: usize,
    item_list: ListState,
    selected: HashSet<String>,
    screen: Screen,
    project: Option<PathBuf>,
    prompt: String,
    prompt_return: Screen,
    flash: String,
}

impl App {
    fn new(project: Option<PathBuf>) -> Result<Self> {
        let harnesses = load_harnesses()?;
        let mut harness_list = ListState::default();
        harness_list.select(Some(0));
        Ok(Self {
            harnesses,
            harness_idx: 0,
            harness_list,
            items: Vec::new(),
            item_idx: 0,
            item_list: ListState::default(),
            selected: HashSet::new(),
            screen: Screen::Harnesses,
            project,
            prompt: String::new(),
            prompt_return: Screen::Harnesses,
            flash: String::new(),
        })
    }

    fn current_harness(&self) -> &Harness {
        &self.harnesses[self.harness_idx]
    }

    fn select_harness(&mut self, idx: usize) {
        self.harness_idx = idx.min(self.harnesses.len().saturating_sub(1));
        self.harness_list.select(Some(self.harness_idx));
    }

    fn select_item(&mut self, idx: usize) {
        if self.items.is_empty() {
            self.item_idx = 0;
            self.item_list.select(None);
            return;
        }
        self.item_idx = idx.min(self.items.len() - 1);
        self.item_list.select(Some(self.item_idx));
    }

    fn open_harness(&mut self) -> Result<()> {
        let needs_project = self.current_harness().project;
        if needs_project && self.project.is_none() {
            self.prompt.clear();
            self.prompt_return = Screen::Items;
            self.screen = Screen::ProjectPrompt;
            return Ok(());
        }
        self.reload_items()?;
        self.selected.clear();
        self.select_item(0);
        self.screen = Screen::Items;
        Ok(())
    }

    fn reload_items(&mut self) -> Result<()> {
        let project = self.project.as_deref();
        let harness = self.current_harness().clone();
        let items = items_for(&harness, project)?;
        self.items = items
            .into_iter()
            .map(|item| {
                let state = classify(&item);
                Row { item, state }
            })
            .collect();
        Ok(())
    }

    fn apply_link(&mut self) -> Result<()> {
        let keys = self.keys_for(LinkState::Missing);
        self.run_on(&keys, |item| link_item(item).to_string())?;
        Ok(())
    }

    fn apply_unlink(&mut self) -> Result<()> {
        let keys = self.keys_for(LinkState::Linked);
        self.run_on(&keys, |item| unlink_item(item).to_string())?;
        Ok(())
    }

    fn keys_for(&self, empty_state: LinkState) -> HashSet<String> {
        if !self.selected.is_empty() {
            return self.selected.clone();
        }
        self.items
            .iter()
            .filter(|r| r.state == empty_state)
            .map(|r| r.item.key())
            .collect()
    }

    fn run_on(&mut self, keys: &HashSet<String>, f: impl Fn(&Item) -> String) -> Result<()> {
        if keys.is_empty() {
            self.flash = "nothing to do".into();
            return Ok(());
        }
        let mut msgs = Vec::new();
        for row in &self.items {
            if keys.contains(&row.item.key()) {
                msgs.push(format!("{}: {}", row.item.key(), f(&row.item)));
            }
        }
        self.flash = msgs.join("  |  ");
        self.reload_items()?;
        Ok(())
    }

    fn toggle(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let row = &self.items[self.item_idx];
        if !row.state.selectable() {
            self.flash = format!("cannot select {}: {}", row.state, row.item.dest.display());
            return;
        }
        let key = row.item.key();
        if !self.selected.remove(&key) {
            self.selected.insert(key);
        }
    }

    fn select_all(&mut self) {
        self.selected = self
            .items
            .iter()
            .filter(|r| r.state.selectable())
            .map(|r| r.item.key())
            .collect();
    }

    fn commit_project(&mut self) -> Result<()> {
        let typed = self.prompt.trim();
        if typed.is_empty() {
            self.screen = Screen::Harnesses;
            return Ok(());
        }
        let path = expand_path(typed);
        self.project = Some(path);
        self.prompt.clear();
        match self.prompt_return {
            Screen::Items => self.open_harness(),
            _ => {
                self.screen = Screen::Harnesses;
                Ok(())
            }
        }
    }

    fn handle(&mut self, key: KeyEvent) -> Result<bool> {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }
        self.flash.clear();
        match self.screen {
            Screen::ProjectPrompt => match key.code {
                KeyCode::Esc => {
                    self.screen = Screen::Harnesses;
                    self.prompt.clear();
                }
                KeyCode::Enter => self.commit_project()?,
                KeyCode::Backspace => {
                    self.prompt.pop();
                }
                KeyCode::Char(c) => self.prompt.push(c),
                _ => {}
            },
            Screen::Harnesses => match key.code {
                KeyCode::Char('q') => return Ok(true),
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_harness(self.harness_idx.saturating_sub(1));
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.select_harness(self.harness_idx + 1);
                }
                KeyCode::Char('p') => {
                    self.prompt = self
                        .project
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default();
                    self.prompt_return = Screen::Harnesses;
                    self.screen = Screen::ProjectPrompt;
                }
                KeyCode::Enter => self.open_harness()?,
                _ => {}
            },
            Screen::Items => match key.code {
                KeyCode::Char('q') => return Ok(true),
                KeyCode::Esc | KeyCode::Char('b') => self.screen = Screen::Harnesses,
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_item(self.item_idx.saturating_sub(1));
                }
                KeyCode::Down | KeyCode::Char('j') => self.select_item(self.item_idx + 1),
                KeyCode::Char(' ') => self.toggle(),
                KeyCode::Char('a') => self.select_all(),
                KeyCode::Char('l') => self.apply_link()?,
                KeyCode::Char('u') => self.apply_unlink()?,
                _ => {}
            },
        }
        Ok(false)
    }
}

pub fn run(project: Option<PathBuf>) -> Result<()> {
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, project);
    ratatui::restore();
    result
}

fn run_app(terminal: &mut DefaultTerminal, project: Option<PathBuf>) -> Result<()> {
    let mut app = App::new(project)?;
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
    let inner = Layout::vertical([Constraint::Percentage(100)])
        .margin(2)
        .split(frame.area())[0];
    let chunks = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(5),
        Constraint::Length(1),
    ])
    .split(inner);

    let project = match &app.project {
        Some(p) => p.display().to_string(),
        None => "(none — press p)".into(),
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(" SERVE ", theme::ink_bold())),
            Line::from(Span::styled(
                format!("pantry: {}", pantry_root().display()),
                Style::default().fg(theme::muted()),
            )),
            Line::from(Span::styled(
                format!("tool: {}", tool_root().display()),
                Style::default().fg(theme::muted()),
            )),
            Line::from(Span::styled(
                format!("project: {project}"),
                Style::default().fg(theme::muted()),
            )),
        ])
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::muted())),
        ),
        chunks[0],
    );

    match app.screen {
        Screen::Harnesses => {
            let items: Vec<ListItem> = app
                .harnesses
                .iter()
                .map(|h| {
                    let tag = if h.project && app.project.is_none() {
                        "  [needs project]"
                    } else {
                        ""
                    };
                    ListItem::new(format!("{}   {}{tag}", h.label, h.kind_names()))
                })
                .collect();
            let list = List::new(items)
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme::muted()))
                        .title(" HARNESSES ")
                        .title_style(theme::ink_bold()),
                )
                .highlight_style(theme::highlight());
            frame.render_stateful_widget(list, chunks[1], &mut app.harness_list);
        }
        Screen::Items => {
            let items: Vec<ListItem> = if app.items.is_empty() {
                vec![ListItem::new(Span::styled(
                    "(no installable items)",
                    Style::default()
                        .fg(theme::muted())
                        .add_modifier(Modifier::ITALIC),
                ))]
            } else {
                app.items
                    .iter()
                    .map(|row| {
                        let mark = if app.selected.contains(&row.item.key()) {
                            "*"
                        } else {
                            " "
                        };
                        let color = match row.state {
                            LinkState::Linked => theme::sage(),
                            LinkState::Missing => theme::heat(),
                            LinkState::Blocked => theme::charred(),
                            LinkState::Foreign => theme::muted(),
                        };
                        ListItem::new(Line::from(vec![
                            Span::raw(format!(
                                "[{mark}] {:<11} {:<28} ",
                                row.item.kind, row.item.name
                            )),
                            Span::styled(
                                format!("{:<8}", row.state.as_str()),
                                Style::default().fg(color),
                            ),
                            Span::raw(format!(" {}", row.item.dest.display())),
                        ]))
                    })
                    .collect()
            };
            let title = format!(" {} ", app.current_harness().label);
            let list = List::new(items)
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme::muted()))
                        .title(title)
                        .title_style(theme::ink_bold()),
                )
                .highlight_style(theme::highlight());
            frame.render_stateful_widget(list, chunks[1], &mut app.item_list);
        }
        Screen::ProjectPrompt => {
            frame.render_widget(
                Paragraph::new(app.prompt.as_str())
                    .style(theme::ink())
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(theme::heat()))
                            .title(" PROJECT ")
                            .title_style(theme::ink_bold()),
                    ),
                chunks[1],
            );
        }
    }

    let help = if !app.flash.is_empty() {
        Line::from(Span::styled(
            truncate(&app.flash, 100),
            Style::default().fg(theme::heat()),
        ))
    } else {
        Line::from(Span::styled(
            help_text(&app.screen),
            Style::default().fg(theme::muted()),
        ))
    };
    frame.render_widget(Paragraph::new(help), chunks[2]);
}

fn help_text(screen: &Screen) -> &'static str {
    match screen {
        Screen::Harnesses => "[ENTER]:Open  [J/K]:Move  [P]:Project  [Q]:Quit",
        Screen::Items => "[SPACE]:Toggle  [A]:All  [L]:Link  [U]:Unlink  [ESC]:Back  [Q]:Quit",
        Screen::ProjectPrompt => "[ENTER]:Confirm  [ESC]:Cancel",
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let take = max.saturating_sub(1);
        format!("{}…", s.chars().take(take).collect::<String>())
    }
}
