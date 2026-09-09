//! Browse skills, rules, extensions, and recipes. Preview only — link via serve.

use std::fs;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, List, ListItem, ListState, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use kitchen::item::{self, PantryEntry};
use kitchen::kitchen::{Kitchen, Recipe};

use crate::theme;

const PREVIEW_CAP: usize = 12_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Skills,
    Rules,
    Extensions,
    Recipes,
}

impl Kind {
    fn id(self) -> &'static str {
        match self {
            Self::Skills => "skills",
            Self::Rules => "rules",
            Self::Extensions => "extensions",
            Self::Recipes => "recipes",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Skills => Self::Rules,
            Self::Rules => Self::Extensions,
            Self::Extensions => Self::Recipes,
            Self::Recipes => Self::Skills,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Skills => Self::Recipes,
            Self::Rules => Self::Skills,
            Self::Extensions => Self::Rules,
            Self::Recipes => Self::Extensions,
        }
    }
}

struct App {
    kind: Kind,
    entries: Vec<PantryEntry>,
    recipes: Vec<Recipe>,
    list: ListState,
    preview: String,
    preview_scroll: u16,
}

impl App {
    fn new() -> Result<Self> {
        let kitchen = Kitchen::load()?;
        let mut app = Self {
            kind: Kind::Skills,
            entries: Vec::new(),
            recipes: kitchen.recipes,
            list: ListState::default(),
            preview: String::new(),
            preview_scroll: 0,
        };
        app.reload()?;
        Ok(app)
    }

    fn names(&self) -> Vec<String> {
        match self.kind {
            Kind::Recipes => self.recipes.iter().map(|r| r.name.clone()).collect(),
            _ => self.entries.iter().map(|e| e.name.clone()).collect(),
        }
    }

    fn reload(&mut self) -> Result<()> {
        self.entries = match self.kind {
            Kind::Recipes => Vec::new(),
            k => item::list_kind(k.id())?,
        };
        let n = self.names().len();
        if n == 0 {
            self.list.select(None);
        } else {
            let i = self.list.selected().unwrap_or(0).min(n - 1);
            self.list.select(Some(i));
        }
        self.preview_scroll = 0;
        self.refresh_preview();
        Ok(())
    }

    fn set_kind(&mut self, kind: Kind) -> Result<()> {
        self.kind = kind;
        self.list.select(Some(0));
        self.reload()
    }

    fn move_list(&mut self, delta: i32) {
        let n = self.names().len();
        if n == 0 {
            return;
        }
        let cur = self.list.selected().unwrap_or(0) as i32;
        let next = (cur + delta).clamp(0, n as i32 - 1) as usize;
        self.list.select(Some(next));
        self.preview_scroll = 0;
        self.refresh_preview();
    }

    fn refresh_preview(&mut self) {
        self.preview = match self.kind {
            Kind::Recipes => recipe_preview(self.list.selected().and_then(|i| self.recipes.get(i))),
            _ => match self.list.selected().and_then(|i| self.entries.get(i)) {
                Some(e) => read_preview(&e.preview),
                None => empty_kind_msg(self.kind).into(),
            },
        };
    }

    fn handle(&mut self, key: KeyEvent) -> Result<bool> {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }
        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                self.set_kind(self.kind.next())?;
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                self.set_kind(self.kind.prev())?;
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_list(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_list(1),
            KeyCode::PageUp => {
                self.preview_scroll = self.preview_scroll.saturating_sub(8);
            }
            KeyCode::PageDown => {
                self.preview_scroll = self.preview_scroll.saturating_add(8);
            }
            _ => {}
        }
        Ok(false)
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
        if !event::poll(Duration::from_secs(60 * 60))? {
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

fn draw(frame: &mut Frame, app: &mut App) {
    let inner = Layout::vertical([Constraint::Percentage(100)])
        .margin(2)
        .split(frame.area())[0];
    let chunks = Layout::vertical([Constraint::Min(5), Constraint::Length(1)]).split(inner);
    let cols = Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(chunks[0]);

    draw_kinds(frame, app, cols[0]);
    draw_preview(frame, app, cols[1]);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "[TAB]:Kind  [J/K]:Move  [PGUP/DN]:Preview  [Q]:Quit  ·  kitchen serve to link",
            Style::default().fg(theme::muted()),
        ))),
        chunks[1],
    );
}

fn draw_kinds(frame: &mut Frame, app: &mut App, area: Rect) {
    frame.render_widget(Clear, area);
    let title = format!(" PANTRY · {} ", app.kind.id());
    let names = app.names();
    let items: Vec<ListItem> = if names.is_empty() {
        vec![ListItem::new(Span::styled(
            empty_kind_msg(app.kind),
            Style::default()
                .fg(theme::muted())
                .add_modifier(Modifier::ITALIC),
        ))]
    } else {
        names.into_iter().map(ListItem::new).collect()
    };
    let list = List::new(items)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::heat()))
                .title(title)
                .title_style(theme::ink_bold()),
        )
        .highlight_style(theme::highlight());
    frame.render_stateful_widget(list, area, &mut app.list);
}

fn draw_preview(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::muted()))
        .title(" PREVIEW ")
        .title_style(theme::ink_bold());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let width = inner.width as usize;
    let height = inner.height as usize;
    if width == 0 || height == 0 {
        return;
    }
    let rows = visual_rows(&app.preview, width);
    let max_scroll = rows.len().saturating_sub(height);
    let start = (app.preview_scroll as usize).min(max_scroll);
    let mut visible: Vec<Line> = rows
        .into_iter()
        .skip(start)
        .take(height)
        .map(Line::from)
        .collect();
    while visible.len() < height {
        visible.push(Line::from(" ".repeat(width)));
    }
    frame.render_widget(Paragraph::new(visible).style(theme::ink()), inner);
}

/// Wrap to `width` and pad every row with spaces so leftover cells cannot ghost.
fn visual_rows(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let text = expand_tabs(text, 4);
    let mut rows = Vec::new();
    for line in text.split('\n') {
        if line.is_empty() {
            rows.push(" ".repeat(width));
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        for chunk in chars.chunks(width) {
            let mut row: String = chunk.iter().collect();
            while row.chars().count() < width {
                row.push(' ');
            }
            rows.push(row);
        }
    }
    rows
}

fn expand_tabs(text: &str, tab: usize) -> String {
    let mut out = String::new();
    let mut col = 0usize;
    for c in text.chars() {
        if c == '\t' {
            let n = tab - (col % tab);
            out.extend(std::iter::repeat_n(' ', n));
            col += n;
        } else if c == '\n' {
            out.push('\n');
            col = 0;
        } else {
            out.push(c);
            col += 1;
        }
    }
    out
}

fn recipe_preview(recipe: Option<&Recipe>) -> String {
    let Some(r) = recipe else {
        return "no recipes.\na recipe is a named plate of skills + rules.".into();
    };
    let skills = if r.skills.is_empty() {
        "—".to_string()
    } else {
        r.skills.join(", ")
    };
    let rules = if r.rules.is_empty() {
        "—".to_string()
    } else {
        r.rules.join(", ")
    };
    format!("{}\n\nskills  {skills}\nrules   {rules}", r.name)
}

fn read_preview(path: &std::path::Path) -> String {
    match fs::read_to_string(path) {
        Ok(raw) => {
            if raw.trim().is_empty() {
                "(empty)".into()
            } else if raw.chars().count() > PREVIEW_CAP {
                let s: String = raw.chars().take(PREVIEW_CAP).collect();
                format!("{s}…")
            } else {
                raw
            }
        }
        Err(_) => format!("cannot read {}", path.display()),
    }
}

fn empty_kind_msg(kind: Kind) -> &'static str {
    match kind {
        Kind::Skills => "no skills in skills/",
        Kind::Rules => "no rules in rules/",
        Kind::Extensions => "no extensions in extensions/",
        Kind::Recipes => "no recipes",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_rows_wraps_and_pads() {
        assert_eq!(
            visual_rows("hello", 3),
            vec!["hel".to_string(), "lo ".to_string()]
        );
    }

    #[test]
    fn visual_rows_blank_line_is_spaces() {
        let rows = visual_rows("a\n\nb", 2);
        assert_eq!(
            rows,
            vec!["a ".to_string(), "  ".to_string(), "b ".to_string()]
        );
    }

    #[test]
    fn expand_tabs_to_columns() {
        assert_eq!(expand_tabs("a\tb", 4), "a   b");
    }
}
