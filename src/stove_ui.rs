//! Local burners and the model catalog. Same stove as chat uses.

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};

use slop_chef::stove::{self, Burner, BurnerStatus, Heat};

use crate::theme;

struct App {
    stove: Vec<BurnerStatus>,
    burner_list: ListState,
    flash: String,
}

impl App {
    fn new() -> Result<Self> {
        let stove = stove::status_all()?;
        let mut burner_list = ListState::default();
        if !stove.is_empty() {
            burner_list.select(Some(0));
        }
        Ok(Self {
            stove,
            burner_list,
            flash: String::new(),
        })
    }

    fn refresh(&mut self) -> Result<()> {
        self.stove = stove::status_all()?;
        if self.stove.is_empty() {
            self.burner_list.select(None);
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

    fn refresh_quiet(&mut self) {
        match self.refresh() {
            Ok(()) => {
                if self.flash.starts_with("warming ")
                    && !self.stove.iter().any(|s| s.heat() == Heat::Warming)
                {
                    self.flash.clear();
                }
            }
            Err(e) => self.flash = e.to_string(),
        }
    }

    fn selected(&self) -> Option<Burner> {
        self.burner_list
            .selected()
            .and_then(|i| self.stove.get(i))
            .map(|s| s.burner)
    }

    fn select(&mut self, idx: usize) {
        if self.stove.is_empty() {
            self.burner_list.select(None);
            return;
        }
        self.burner_list.select(Some(idx.min(self.stove.len() - 1)));
    }

    fn simmer(&mut self) {
        match self.selected() {
            Some(b) => match stove::simmer(b) {
                Ok(msg) => {
                    self.refresh_quiet();
                    self.flash = msg;
                }
                Err(e) => {
                    self.refresh_quiet();
                    self.flash = e.to_string();
                }
            },
            None => self.flash = "pick a burner".into(),
        }
    }

    fn cool(&mut self) {
        match self.selected() {
            Some(b) => match stove::cool(b) {
                Ok(msg) => {
                    self.refresh_quiet();
                    self.flash = msg;
                }
                Err(e) => {
                    self.refresh_quiet();
                    self.flash = e.to_string();
                }
            },
            None => self.flash = "pick a burner".into(),
        }
    }

    fn handle(&mut self, key: KeyEvent) -> Result<bool> {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }
        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.burner_list.selected().unwrap_or(0).saturating_sub(1);
                self.select(i);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.burner_list.selected().unwrap_or(0).saturating_add(1);
                self.select(i);
            }
            KeyCode::Enter | KeyCode::Char('s') => self.simmer(),
            KeyCode::Char('c') => self.cool(),
            KeyCode::Char('r') => {
                self.refresh_quiet();
                self.flash = "stove checked".into();
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
        if needs_poll(&app) {
            app.refresh_quiet();
        }
        terminal.draw(|frame| draw(frame, &mut app))?;
        let wait = if needs_poll(&app) {
            Duration::from_millis(250)
        } else {
            Duration::from_secs(60 * 60)
        };
        if !event::poll(wait)? {
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

fn needs_poll(app: &App) -> bool {
    app.stove.iter().any(|s| s.heat() == Heat::Warming) || app.flash.starts_with("warming ")
}

fn draw(frame: &mut Frame, app: &mut App) {
    let inner = Layout::vertical([Constraint::Percentage(100)])
        .margin(2)
        .split(frame.area())[0];
    let chunks = Layout::vertical([Constraint::Min(5), Constraint::Length(1)]).split(inner);
    let cols = Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(chunks[0]);

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
                .title(" STOVE ")
                .title_style(theme::ink_bold()),
        )
        .highlight_style(theme::highlight());
    frame.render_stateful_widget(list, cols[0], &mut app.burner_list);

    let detail = if let Some(st) = app.burner_list.selected().and_then(|i| app.stove.get(i)) {
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
                .title(" BURNER ")
                .title_style(theme::ink_bold()),
        ),
        cols[1],
    );

    let help = if app.flash.is_empty() {
        Line::from(Span::styled(
            "[ENTER]:Simmer  [C]:Cool  [R]:Refresh  [Q]:Quit",
            Style::default().fg(theme::muted()),
        ))
    } else {
        Line::from(Span::styled(
            truncate(&app.flash, 80),
            Style::default().fg(theme::heat()),
        ))
    };
    frame.render_widget(Paragraph::new(help), chunks[1]);
}

fn burner_line(st: &BurnerStatus) -> Line<'static> {
    let heat = st.heat();
    Line::from(vec![
        Span::styled(format!("{:<8}", st.burner.label()), theme::ink_bold()),
        Span::raw("  "),
        Span::styled(format!("{:<10}", heat.label()), heat_style(heat)),
        Span::styled(format!(":{}", st.port), Style::default().fg(theme::muted())),
    ])
}

fn burner_detail(st: &BurnerStatus) -> Vec<Line<'static>> {
    let heat = st.heat();
    let pid = st.pid.map(|p| p.to_string()).unwrap_or_else(|| "—".into());
    let owner = match heat {
        Heat::Cold => "—".to_string(),
        _ if st.managed => "slop-chef".to_string(),
        _ => "external".to_string(),
    };
    let url = format!("127.0.0.1:{}", st.port);
    let log = ellipsize_path(&stove::log_path(st.burner), 40);
    let mut lines = vec![
        Line::from(Span::styled(st.burner.label(), theme::ink_bold())),
        Line::from(""),
        kv_styled("heat", heat.label().to_string(), heat_style(heat)),
        kv("port", st.port.to_string()),
        kv("pid", pid),
        kv("started by", owner),
        kv("url", url),
        kv_styled("log", log, Style::default().fg(theme::muted())),
        Line::from(""),
        Line::from(Span::styled("models", Style::default().fg(theme::muted()))),
    ];
    match stove::served_models(st.burner, st) {
        Ok(models) if models.is_empty() => {
            lines.push(Line::from(Span::styled(
                "—",
                Style::default().fg(theme::muted()),
            )));
        }
        Ok(models) => {
            for m in models {
                lines.push(Line::from(Span::styled(model_label(&m), theme::ink())));
            }
        }
        Err(e) => lines.push(Line::from(Span::styled(
            e.to_string(),
            Style::default().fg(theme::charred()),
        ))),
    }
    lines
}

fn heat_style(heat: Heat) -> Style {
    match heat {
        Heat::Simmering => Style::default().fg(theme::heat()),
        Heat::Warming => Style::default()
            .fg(theme::heat())
            .add_modifier(Modifier::ITALIC),
        Heat::Cold => Style::default().fg(theme::muted()),
    }
}

fn kv(k: &str, v: String) -> Line<'static> {
    kv_styled(k, v, theme::ink())
}

fn kv_styled(k: &str, v: String, v_style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{k:<14}"), Style::default().fg(theme::muted())),
        Span::styled(v, v_style),
    ])
}

fn model_label(model: &str) -> String {
    let name = model
        .trim_end_matches('/')
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(model);
    truncate(name, 48)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let take = max.saturating_sub(1);
        format!("{}…", s.chars().take(take).collect::<String>())
    }
}

fn ellipsize_path(path: &std::path::Path, max: usize) -> String {
    let raw = shorten_home(path);
    if raw.chars().count() <= max {
        return raw;
    }
    format!("{}…", truncate(&raw, max.saturating_sub(1)))
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
