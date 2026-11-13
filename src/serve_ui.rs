//! Harness symlink UI. Same pantry, different door — link, don't browse.

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, List, ListItem, ListState, Paragraph, Tabs};
use ratatui::Frame;

use chef::config::{expand_path, Harness};
use chef::item::{classify, items_for, Item, LinkState};
use chef::load_harnesses;
use chef::ops::{link_item, unlink_item};

use crate::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Harnesses,
    Items,
    ProjectPrompt,
}

struct Row {
    item: Item,
    state: LinkState,
}

/// Single source of truth for the unifyable top bar; pantry_ui::Kind order
/// and ids must match these.
pub const TAB_TITLES: [&str; 4] = ["skills", "rules", "extensions", "serve"];
pub const TAB_SERVE_IDX: usize = 3;

pub struct App {
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
    help: bool,
    unlink_confirmation: bool,
}

impl App {
    pub fn new(project: Option<PathBuf>) -> Result<Self> {
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
            help: false,
            unlink_confirmation: false,
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
        if self.selected.is_empty() {
            self.flash = "select items first; a selects all".into();
            return Ok(());
        }
        let keys = self.keys_for(LinkState::Missing);
        self.run_on(&keys, |item| link_item(item).to_string())?;
        Ok(())
    }

    fn apply_unlink(&mut self) -> Result<()> {
        if self.selected.is_empty() {
            self.flash = "select items first; a selects all".into();
            return Ok(());
        }
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
        let selectable: HashSet<String> = self
            .items
            .iter()
            .filter(|r| r.state.selectable())
            .map(|r| r.item.key())
            .collect();
        if !selectable.is_empty() && self.selected == selectable {
            self.selected.clear();
        } else {
            self.selected = selectable;
        }
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

    /// Modal input in progress: keys typed here are content, not commands.
    pub fn busy(&self) -> bool {
        self.help || self.unlink_confirmation || self.screen == Screen::ProjectPrompt
    }

    pub fn handle(&mut self, key: KeyEvent) -> Result<bool> {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }
        if key.code == KeyCode::Char('?') && !self.unlink_confirmation {
            self.help = !self.help;
            return Ok(false);
        }
        if self.help {
            self.help = false;
            return Ok(false);
        }
        self.flash.clear();
        if self.unlink_confirmation {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.unlink_confirmation = false;
                    self.apply_unlink()?;
                }
                KeyCode::Char('n')
                | KeyCode::Char('N')
                | KeyCode::Esc
                | KeyCode::Char('q')
                | KeyCode::Char('Q') => {
                    self.unlink_confirmation = false;
                    self.flash = "unlink cancelled".into();
                }
                _ => {}
            }
            return Ok(false);
        }
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
                KeyCode::Char('w') => self.apply_link()?,
                KeyCode::Char('u') => {
                    if self.selected.is_empty() {
                        self.flash = "select items first; a selects all".into();
                    } else if self.keys_for(LinkState::Linked).is_empty() {
                        self.flash = "nothing to unlink".into();
                    } else {
                        self.unlink_confirmation = true;
                    }
                }
                _ => {}
            },
        }
        Ok(false)
    }
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let inner = Rect {
        x: area.x.saturating_add(2),
        y: area.y,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(1),
    ])
    .split(inner);

    let tabs = Tabs::new(TAB_TITLES.map(Line::from))
        .select(TAB_SERVE_IDX)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::heat()))
                .title(" CHEF ")
                .title_style(theme::ink_bold()),
        )
        .style(Style::default().fg(theme::muted()))
        .highlight_style(
            Style::default()
                .fg(theme::heat())
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .divider("│")
        .padding(" ", " ");
    frame.render_widget(tabs, chunks[0]);

    match app.screen {
        Screen::Harnesses => {
            let cols = Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)])
                .split(chunks[1]);
            // Group rows by harness family (cursor, claude, …), with a small
            // non-selectable header between families.
            let mut items: Vec<ListItem> = Vec::with_capacity(app.harnesses.len() + 3);
            let mut row_of_harness: Vec<usize> = Vec::with_capacity(app.harnesses.len());
            let mut prev_group: Option<String> = None;
            for h in &app.harnesses {
                let group = h.id.split('-').next().unwrap_or(&h.id).to_string();
                let same = prev_group.as_deref() == Some(group.as_str());
                prev_group = Some(group.clone());
                if !same {
                    items.push(ListItem::new(""));
                    items.push(ListItem::new(Span::styled(
                        format!("── {} ──", capitalize(&group)),
                        Style::default()
                            .fg(theme::muted())
                            .add_modifier(Modifier::ITALIC),
                    )));
                }
                row_of_harness.push(items.len());
                let tag = if h.project && app.project.is_none() {
                    "  [needs project]"
                } else {
                    ""
                };
                items.push(ListItem::new(format!(
                    "{}   {}{tag}",
                    h.label,
                    h.kind_names()
                )));
            }
            app.harness_list
                .select(Some(row_of_harness[app.harness_idx]));
            let list = List::new(items)
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme::muted()))
                        .title(" HARNESSES ")
                        .title_style(theme::ink_bold()),
                )
                .highlight_style(theme::highlight());
            frame.render_stateful_widget(list, cols[0], &mut app.harness_list);

            let harness = app.current_harness();
            let project = app
                .project
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "not configured".into());
            let details = vec![
                Line::from(Span::styled(harness.label.clone(), theme::ink_bold())),
                Line::from(Span::styled(
                    family_desc(&harness.id).to_string(),
                    Style::default().fg(theme::muted()),
                )),
                Line::from(""),
                Line::from(format!("Provides: {}", harness.kind_names())),
                Line::from(format!(
                    "Project harness: {}",
                    if harness.project { "yes" } else { "no" }
                )),
                Line::from(format!("Project path: {project}")),
                Line::from(""),
                Line::from(Span::styled(
                    "Press ENTER to manage links.",
                    Style::default().fg(theme::muted()),
                )),
            ];
            frame.render_widget(
                Paragraph::new(details)
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(theme::muted()))
                            .title(" DETAILS ")
                            .title_style(theme::ink_bold()),
                    )
                    .style(theme::ink()),
                cols[1],
            );
        }
        Screen::Items => {
            let cols = Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)])
                .split(chunks[1]);
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
                            // Columns: selection, link status, kind, name.
                            // Status leads so it survives narrow panes; the
                            // full destination lives in the DETAILS pane.
                            Span::raw(format!("[{mark}] ")),
                            Span::styled(
                                format!("{:<9}", row.state.as_str()),
                                Style::default().fg(color),
                            ),
                            Span::raw(format!("{:<10} {}", row.item.kind, row.item.name)),
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
            frame.render_stateful_widget(list, cols[0], &mut app.item_list);

            let details = app
                .items
                .get(app.item_idx)
                .map(|row| {
                    vec![
                        Line::from(Span::styled(row.item.name.clone(), theme::ink_bold())),
                        Line::from(""),
                        Line::from(format!("Kind: {}", row.item.kind)),
                        Line::from(format!("State: {}", row.state)),
                        Line::from(""),
                        Line::from(Span::styled("Source", theme::ink_bold())),
                        Line::from(row.item.source.display().to_string()),
                        Line::from(""),
                        Line::from(Span::styled("Destination", theme::ink_bold())),
                        Line::from(row.item.dest.display().to_string()),
                    ]
                })
                .unwrap_or_else(|| vec![Line::from("No item selected")]);
            frame.render_widget(
                Paragraph::new(details)
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(theme::muted()))
                            .title(" DETAILS ")
                            .title_style(theme::ink_bold()),
                    )
                    .style(theme::ink()),
                cols[1],
            );
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
        match pending_verdict(app) {
            Some(verdict) => Line::from(Span::styled(
                format!("{verdict} | {}", help_text(&app.screen)),
                Style::default().fg(theme::sage()),
            )),
            None => Line::from(Span::styled(
                help_text(&app.screen),
                Style::default().fg(theme::muted()),
            )),
        }
    };
    frame.render_widget(Paragraph::new(help), chunks[2]);

    if app.unlink_confirmation {
        let area = frame.area();
        let width = 58.min(area.width.saturating_sub(4));
        let height = 7.min(area.height.saturating_sub(2));
        let popup = Rect {
            x: area.x + area.width.saturating_sub(width) / 2,
            y: area.y + area.height.saturating_sub(height) / 2,
            width,
            height,
        };
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new("Unlink selected harness links?\n\nOnly destination symlinks are removed.\n[Y] Continue  [N/Esc/Q] Cancel")
                .block(Block::bordered().border_type(BorderType::Rounded).title(" Confirm unlink ").title_style(theme::ink_bold()))
                .style(theme::ink()),
            popup,
        );
    }
    if app.help {
        crate::pantry_ui::draw_help(frame);
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// One-line plain explanation of what a harness family actually is.
fn family_desc(harness_id: &str) -> &'static str {
    match harness_id.split('-').next().unwrap_or(harness_id) {
        "cursor" => "Cursor editor links (user + project).",
        "claude" => "Claude Code links (user + project).",
        "pi" => "Pi coding agent links into ~/.pi/agent.",
        "agents" => "Generic user-level skills dir shared by agent CLIs.",
        _ => "",
    }
}

/// What pressing `w` / `u` would do right now: same target set as the
/// apply fns (selection when non-empty, otherwise all matching items).
/// None when there is nothing pending.
fn pending_verdict(app: &App) -> Option<String> {
    let rows: Vec<&Row> = if app.selected.is_empty() {
        app.items.iter().collect()
    } else {
        app.items
            .iter()
            .filter(|r| app.selected.contains(&r.item.key()))
            .collect()
    };
    let to_link = rows
        .iter()
        .filter(|r| r.state == LinkState::Missing)
        .count();
    let to_unlink = rows
        .iter()
        .filter(|r| matches!(r.state, LinkState::Linked | LinkState::Foreign))
        .count();
    match (to_link, to_unlink) {
        (0, 0) => None,
        (l, u) => Some(format!("{l} to link · {u} to unlink")),
    }
}

fn help_text(screen: &Screen) -> &'static str {
    match screen {
        Screen::Harnesses => "[enter]:open  [j/k]:move  [p]:project  [?]:help  [q]:quit",
        Screen::Items => {
            "[space]:toggle  [a]:all  [w]:link  [u]:unlink  [esc]:back  [j/k]:move  [?]:help  [q]:quit"
        }
        Screen::ProjectPrompt => "[enter]:confirm  [esc]:cancel",
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
