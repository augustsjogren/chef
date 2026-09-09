//! Browse skills, rules, and extensions across repo + harness dirs. Preview only.

use std::fs;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, List, ListItem, ListState, Paragraph, Tabs};
use ratatui::{DefaultTerminal, Frame};

use kitchen::import::import_skill;
use kitchen::item::{self, PantryEntry};

use crate::theme;

const PREVIEW_CAP: usize = 12_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Skills,
    Rules,
    Extensions,
}

impl Kind {
    const ALL: [Kind; 3] = [Self::Skills, Self::Rules, Self::Extensions];

    fn id(self) -> &'static str {
        match self {
            Self::Skills => "skills",
            Self::Rules => "rules",
            Self::Extensions => "extensions",
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|&k| k == self).unwrap()
    }

    fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    fn prev(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

struct App {
    kind: Kind,
    all_sources: bool,
    entries: Vec<PantryEntry>,
    list: ListState,
    preview: String,
    preview_md: bool,
    preview_scroll: u16,
    flash: String,
}

impl App {
    fn new() -> Result<Self> {
        let mut app = Self {
            kind: Kind::Skills,
            all_sources: false,
            entries: Vec::new(),
            list: ListState::default(),
            preview: String::new(),
            preview_md: false,
            preview_scroll: 0,
            flash: String::new(),
        };
        app.reload()?;
        Ok(app)
    }

    fn reload(&mut self) -> Result<()> {
        let keep = self
            .list
            .selected()
            .and_then(|i| self.entries.get(i))
            .map(|e| e.name.clone());
        self.entries = item::list_pantry(self.kind.id(), self.all_sources)?;
        let n = self.entries.len();
        if n == 0 {
            self.list.select(None);
        } else if let Some(name) = keep {
            let i = self
                .entries
                .iter()
                .position(|e| e.name == name)
                .unwrap_or_else(|| self.list.selected().unwrap_or(0).min(n - 1));
            self.list.select(Some(i));
        } else {
            let i = self.list.selected().unwrap_or(0).min(n - 1);
            self.list.select(Some(i));
        }
        self.preview_scroll = 0;
        self.refresh_preview();
        Ok(())
    }

    fn toggle_sources(&mut self) -> Result<()> {
        self.all_sources = !self.all_sources;
        self.reload()
    }

    fn import_selected(&mut self) -> Result<()> {
        if self.kind != Kind::Skills || !self.all_sources {
            self.flash = "import: skills tab + all sources (A)".into();
            return Ok(());
        }
        let Some(entry) = self
            .list
            .selected()
            .and_then(|i| self.entries.get(i))
            .cloned()
        else {
            return Ok(());
        };
        if item::is_kitchen_entry(&entry) {
            self.flash = "already in slop-kitchen".into();
            return Ok(());
        }
        match import_skill(&entry.name, &entry.root, &entry.location, false) {
            Ok(result) => {
                self.flash = if result.relinked.is_some() {
                    format!("imported {} → repo, relinked", entry.name)
                } else {
                    format!("imported {} → repo", entry.name)
                };
                self.reload()?;
            }
            Err(e) => self.flash = e.to_string(),
        }
        Ok(())
    }

    fn set_kind(&mut self, kind: Kind) -> Result<()> {
        self.kind = kind;
        self.list.select(Some(0));
        self.reload()
    }

    fn move_list(&mut self, delta: i32) {
        let n = self.entries.len();
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
        match self.list.selected().and_then(|i| self.entries.get(i)) {
            Some(e) => match read_preview(&e.preview) {
                Ok(raw) if raw.trim().is_empty() => {
                    self.preview = "(empty)".into();
                    self.preview_md = false;
                }
                Ok(raw) => {
                    self.preview = raw;
                    self.preview_md = e.preview.extension().and_then(|x| x.to_str()) == Some("md")
                        || e.preview.extension().and_then(|x| x.to_str()) == Some("mdc");
                }
                Err(msg) => {
                    self.preview = msg;
                    self.preview_md = false;
                }
            },
            None => {
                self.preview = empty_kind_msg(self.kind, self.all_sources).into();
                self.preview_md = false;
            }
        }
    }

    fn handle(&mut self, key: KeyEvent) -> Result<bool> {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }
        if !matches!(key.code, KeyCode::Char('i') | KeyCode::Char('I')) {
            self.flash.clear();
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
            KeyCode::Char('a') | KeyCode::Char('A') => self.toggle_sources()?,
            KeyCode::Char('i') | KeyCode::Char('I') => self.import_selected()?,
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
    let cols = Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(chunks[1]);

    draw_tabs(frame, app, chunks[0]);
    draw_kinds(frame, app, cols[0]);
    draw_preview(frame, app, cols[1]);

    let help = if !app.flash.is_empty() {
        Line::from(Span::styled(
            truncate(&app.flash, 100),
            Style::default().fg(theme::heat()),
        ))
    } else if app.all_sources && app.kind == Kind::Skills {
        Line::from(Span::styled(
            "[TAB]:Kind  [J/K]:Move  [A]:Sources  [I]:Import  [PGUP/DN]:Preview  [Q]:Quit  ·  [k]repo  [a]other",
            Style::default().fg(theme::muted()),
        ))
    } else if app.all_sources {
        Line::from(Span::styled(
            "[TAB]:Kind  [J/K]:Move  [A]:Sources (all)  [PGUP/DN]:Preview  [Q]:Quit  ·  [k]repo  [a]other",
            Style::default().fg(theme::muted()),
        ))
    } else {
        Line::from(Span::styled(
            "[TAB]:Kind  [J/K]:Move  [A]:Sources (repo)  [PGUP/DN]:Preview  [Q]:Quit",
            Style::default().fg(theme::muted()),
        ))
    };
    frame.render_widget(Paragraph::new(help), chunks[2]);
}

fn draw_tabs(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);
    let titles: Vec<Line> = Kind::ALL.iter().map(|k| Line::from(k.id())).collect();
    let tabs = Tabs::new(titles)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::heat()))
                .title(" SLOP-KITCHEN ")
                .title_style(theme::ink_bold()),
        )
        .style(Style::default().fg(theme::muted()))
        .highlight_style(
            Style::default()
                .fg(theme::heat())
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .select(app.kind.index())
        .divider("│")
        .padding(" ", " ");
    frame.render_widget(tabs, area);
}

fn list_item_for_entry(entry: &PantryEntry, all_sources: bool) -> ListItem<'static> {
    if !all_sources {
        return ListItem::new(entry.name.clone());
    }
    let mark = if item::is_kitchen_entry(entry) {
        Span::styled("[k] ", Style::default().fg(theme::sage()))
    } else {
        Span::styled("[a] ", Style::default().fg(theme::muted()))
    };
    ListItem::new(Line::from(vec![mark, Span::raw(entry.name.clone())]))
}

fn draw_kinds(frame: &mut Frame, app: &mut App, area: Rect) {
    frame.render_widget(Clear, area);
    let scope = if app.all_sources { " · all" } else { "" };
    let title = format!(" {}{} ", app.kind.id().to_uppercase(), scope);
    let items: Vec<ListItem> = if app.entries.is_empty() {
        vec![ListItem::new(Span::styled(
            empty_kind_msg(app.kind, app.all_sources),
            Style::default()
                .fg(theme::muted())
                .add_modifier(Modifier::ITALIC),
        ))]
    } else {
        app.entries
            .iter()
            .map(|e| list_item_for_entry(e, app.all_sources))
            .collect()
    };
    let list = List::new(items)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::muted()))
                .title(title)
                .title_style(theme::ink_bold()),
        )
        .highlight_style(theme::highlight());
    frame.render_stateful_widget(list, area, &mut app.list);
}

fn draw_preview(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);
    let loc = app
        .list
        .selected()
        .and_then(|i| app.entries.get(i))
        .map(|e| format!(" @{}", e.location))
        .unwrap_or_default();
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::muted()))
        .title(format!(" PREVIEW{loc} "))
        .title_style(theme::ink_bold());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let width = inner.width as usize;
    let height = inner.height as usize;
    if width == 0 || height == 0 {
        return;
    }
    let rows = if app.preview_md {
        crate::markdown::render(&app.preview, width)
    } else {
        visual_rows(&app.preview, width)
            .into_iter()
            .map(Line::from)
            .collect()
    };
    let max_scroll = rows.len().saturating_sub(height);
    let start = (app.preview_scroll as usize).min(max_scroll);
    let mut visible: Vec<Line> = rows.into_iter().skip(start).take(height).collect();
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

fn read_preview(path: &std::path::Path) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(raw) => {
            if raw.chars().count() > PREVIEW_CAP {
                let s: String = raw.chars().take(PREVIEW_CAP).collect();
                Ok(format!("{s}…"))
            } else {
                Ok(raw)
            }
        }
        Err(_) => Err(format!("cannot read {}", path.display())),
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

fn empty_kind_msg(kind: Kind, all_sources: bool) -> &'static str {
    match (kind, all_sources) {
        (Kind::Skills, false) => "no skills in slop-kitchen",
        (Kind::Skills, true) => "no skills found",
        (Kind::Rules, false) => "no rules in slop-kitchen",
        (Kind::Rules, true) => "no rules found",
        (Kind::Extensions, false) => "no extensions in slop-kitchen",
        (Kind::Extensions, true) => "no extensions found",
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

    #[test]
    fn kind_cycles_all_three() {
        let mut k = Kind::Skills;
        for expected in Kind::ALL {
            assert_eq!(k, expected);
            k = k.next();
        }
        assert_eq!(k, Kind::Skills);
        assert_eq!(Kind::Skills.prev(), Kind::Extensions);
        assert_eq!(Kind::Extensions.next(), Kind::Skills);
    }
}
