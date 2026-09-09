//! Plain local-model chat. Stove picks the burner; this screen is just talk.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};

use slop_chef::chat::{save_transcript, stream_reply, ChatMessage, Transcript};
use slop_chef::stove::{self, Burner, CatalogEntry, Heat};

use crate::theme;

enum Screen {
    Chat,
    Models,
}

enum ChatEvent {
    Warming,
    Delta(String),
    Done,
    Err(String),
}

struct App {
    screen: Screen,
    transcript: Transcript,
    catalog: Vec<CatalogEntry>,
    model_list: ListState,
    input: String,
    flash: String,
    rx: Option<Receiver<ChatEvent>>,
    cancel: Option<Arc<AtomicBool>>,
    scroll: u16,
    stick_bottom: bool,
    heat_text: String,
}

impl App {
    fn new() -> Result<Self> {
        let catalog = stove::model_catalog().unwrap_or_default();
        let transcript = match slop_chef::chat::load_active()? {
            Some(t) => t,
            None => {
                let (burner, model) = default_pick(&catalog);
                Transcript::new(burner, model)
            }
        };
        let mut model_list = ListState::default();
        if let Some(i) = catalog
            .iter()
            .position(|e| e.burner == transcript.burner && e.model == transcript.model)
        {
            model_list.select(Some(i));
        } else if !catalog.is_empty() {
            model_list.select(Some(0));
        }
        let heat_text = heat_label(transcript.burner);
        Ok(Self {
            screen: Screen::Chat,
            transcript,
            catalog,
            model_list,
            input: String::new(),
            flash: String::new(),
            rx: None,
            cancel: None,
            scroll: 0,
            stick_bottom: true,
            heat_text,
        })
    }

    fn streaming(&self) -> bool {
        self.rx.is_some()
    }

    fn refresh_catalog(&mut self) {
        match stove::model_catalog() {
            Ok(c) => {
                self.catalog = c;
                if self.model_list.selected().is_none() && !self.catalog.is_empty() {
                    self.model_list.select(Some(0));
                }
            }
            Err(e) => self.flash = e.to_string(),
        }
    }

    fn start_send(&mut self) {
        if self.streaming() {
            return;
        }
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return;
        }
        self.input.clear();
        self.transcript.messages.push(ChatMessage::user(text));
        self.transcript.messages.push(ChatMessage::assistant(""));
        self.stick_bottom = true;
        self.flash.clear();

        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.rx = Some(rx);
        self.cancel = Some(cancel.clone());

        let burner = self.transcript.burner;
        let model = self.transcript.model.clone();
        let messages = self.transcript.messages.clone();
        // drop the empty assistant placeholder from the request
        let mut messages = messages;
        if messages
            .last()
            .is_some_and(|m| m.role == "assistant" && m.content.is_empty())
        {
            messages.pop();
        }

        std::thread::spawn(move || {
            let _ = tx.send(ChatEvent::Warming);
            let result = stream_reply(burner, &model, &messages, &cancel, |delta| {
                let _ = tx.send(ChatEvent::Delta(delta.to_string()));
            });
            let _ = tx.send(match result {
                Ok(()) => ChatEvent::Done,
                Err(e) => ChatEvent::Err(e.to_string()),
            });
        });
    }

    fn stop_stream(&mut self) {
        if let Some(c) = &self.cancel {
            c.store(true, Ordering::SeqCst);
        }
    }

    fn poll_stream(&mut self) {
        let Some(rx) = self.rx.as_ref() else {
            return;
        };
        let mut events = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }
        if events.is_empty() {
            return;
        }
        for ev in events {
            match ev {
                ChatEvent::Warming => {
                    self.flash = format!("warming {}…", self.transcript.burner.id());
                    self.heat_text = heat_label(self.transcript.burner);
                }
                ChatEvent::Delta(s) => {
                    if self.flash.starts_with("warming ") {
                        self.flash.clear();
                    }
                    if let Some(last) = self.transcript.messages.last_mut() {
                        if last.role == "assistant" {
                            last.content.push_str(&s);
                        }
                    }
                    self.stick_bottom = true;
                }
                ChatEvent::Done => {
                    self.rx = None;
                    self.cancel = None;
                    self.flash.clear();
                    self.heat_text = heat_label(self.transcript.burner);
                    trim_empty_assistant(&mut self.transcript);
                    let _ = save_transcript(&self.transcript);
                }
                ChatEvent::Err(e) => {
                    self.rx = None;
                    self.cancel = None;
                    self.heat_text = heat_label(self.transcript.burner);
                    if e.contains("chat stopped") {
                        self.flash = "stopped".into();
                    } else {
                        self.flash = e;
                    }
                    trim_empty_assistant(&mut self.transcript);
                    let _ = save_transcript(&self.transcript);
                }
            }
        }
    }

    fn new_chat(&mut self) {
        if self.streaming() {
            self.stop_stream();
        }
        if !self.transcript.messages.is_empty() {
            let _ = save_transcript(&self.transcript);
        }
        let (burner, model) = (self.transcript.burner, self.transcript.model.clone());
        self.transcript = Transcript::new(burner, model);
        self.scroll = 0;
        self.stick_bottom = true;
        self.flash = "new chat".into();
        let _ = save_transcript(&self.transcript);
    }

    fn apply_model(&mut self) {
        let Some(i) = self.model_list.selected() else {
            self.flash = "pick a model".into();
            return;
        };
        let Some(entry) = self.catalog.get(i).cloned() else {
            return;
        };
        self.transcript.burner = entry.burner;
        self.transcript.model = entry.model;
        self.heat_text = heat_label(entry.burner);
        self.screen = Screen::Chat;
        self.flash = format!("using {}", model_label(&self.transcript.model));
        let _ = save_transcript(&self.transcript);
    }

    fn handle(&mut self, key: KeyEvent) -> Result<bool> {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return self.handle_ctrl(key);
        }
        match self.screen {
            Screen::Models => self.handle_models(key),
            Screen::Chat => self.handle_chat(key),
        }
    }

    fn handle_ctrl(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('c') => {
                if self.streaming() {
                    self.stop_stream();
                }
                if !self.transcript.messages.is_empty() {
                    let _ = save_transcript(&self.transcript);
                }
                return Ok(true);
            }
            KeyCode::Char('n') => {
                if matches!(self.screen, Screen::Chat) {
                    self.new_chat();
                }
            }
            KeyCode::Char('o') if !self.streaming() => {
                self.refresh_catalog();
                self.screen = Screen::Models;
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_models(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => self.screen = Screen::Chat,
            KeyCode::Enter => self.apply_model(),
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.model_list.selected().unwrap_or(0).saturating_sub(1);
                if !self.catalog.is_empty() {
                    self.model_list.select(Some(i));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.model_list.selected().unwrap_or(0).saturating_add(1);
                if !self.catalog.is_empty() {
                    self.model_list.select(Some(i.min(self.catalog.len() - 1)));
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_chat(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                if self.streaming() {
                    self.stop_stream();
                }
            }
            KeyCode::Enter => self.start_send(),
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::PageUp => {
                self.stick_bottom = false;
                self.scroll = self.scroll.saturating_sub(8);
            }
            KeyCode::PageDown => {
                self.scroll = self.scroll.saturating_add(8);
            }
            KeyCode::Char(c) if !self.streaming() => {
                self.input.push(c);
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
        app.poll_stream();
        terminal.draw(|frame| draw(frame, &mut app))?;
        let wait = if app.streaming() {
            Duration::from_millis(80)
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

fn draw(frame: &mut Frame, app: &mut App) {
    let inner = Layout::vertical([Constraint::Percentage(100)])
        .margin(2)
        .split(frame.area())[0];

    match app.screen {
        Screen::Chat => draw_chat(frame, app, inner),
        Screen::Models => draw_models(frame, app, inner),
    }
}

fn draw_chat(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Min(5),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(area);

    let title = format!(
        " CHAT · {} · {} ",
        model_label(&app.transcript.model),
        app.heat_text
    );
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::muted()))
        .title(title)
        .title_style(theme::ink_bold());
    let inner = block.inner(chunks[0]);
    frame.render_widget(block, chunks[0]);

    let lines = transcript_lines(&app.transcript, app.streaming());
    let max_scroll = lines.len().saturating_sub(inner.height as usize) as u16;
    if app.stick_bottom {
        app.scroll = max_scroll;
    } else {
        app.scroll = app.scroll.min(max_scroll);
    }

    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((app.scroll, 0))
            .style(theme::ink()),
        inner,
    );

    let input_title = if app.streaming() {
        " wait "
    } else {
        " message "
    };
    let input_style = if app.streaming() {
        Style::default().fg(theme::muted())
    } else {
        theme::ink()
    };
    frame.render_widget(
        Paragraph::new(app.input.clone()).style(input_style).block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(if app.streaming() {
                    theme::muted()
                } else {
                    theme::heat()
                }))
                .title(input_title)
                .title_style(theme::ink_bold()),
        ),
        chunks[1],
    );

    let help = if app.streaming() {
        "[ESC]:Stop  [^Q]:Quit"
    } else {
        "[ENTER]:Send  [^O]:Model  [^N]:New  [PGUP/DN]:Scroll  [^Q]:Quit"
    };
    let footer = if !app.flash.is_empty() {
        Line::from(Span::styled(
            truncate(&app.flash, 80),
            Style::default().fg(theme::heat()),
        ))
    } else {
        Line::from(Span::styled(help, Style::default().fg(theme::muted())))
    };
    frame.render_widget(Paragraph::new(footer), chunks[2]);
}

fn draw_models(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(area);
    let items: Vec<ListItem> = if app.catalog.is_empty() {
        vec![ListItem::new(Span::styled(
            "no models — simmer mlx on the stove",
            Style::default()
                .fg(theme::muted())
                .add_modifier(Modifier::ITALIC),
        ))]
    } else {
        app.catalog
            .iter()
            .map(|e| {
                let mark = if e.burner == app.transcript.burner && e.model == app.transcript.model {
                    "·"
                } else {
                    " "
                };
                ListItem::new(Line::from(vec![
                    Span::raw(format!("{mark} ")),
                    Span::styled(format!("{:<7}", e.burner.id()), theme::ink_bold()),
                    Span::styled(format!("{:<10}", e.heat.label()), heat_style(e.heat)),
                    Span::styled(model_label(&e.model), theme::ink()),
                ]))
            })
            .collect()
    };
    let list = List::new(items)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::muted()))
                .title(" MODEL ")
                .title_style(theme::ink_bold()),
        )
        .highlight_style(theme::highlight());
    frame.render_stateful_widget(list, chunks[0], &mut app.model_list);
    frame.render_widget(
        Paragraph::new("[J/K]:Move  [ENTER]:Use  [ESC]:Back")
            .style(Style::default().fg(theme::muted())),
        chunks[1],
    );
}

fn transcript_lines(t: &Transcript, streaming: bool) -> Vec<Line<'static>> {
    if t.messages.is_empty() {
        return vec![Line::from(Span::styled(
            "plain chat. type and enter. first send warms the stove.",
            Style::default()
                .fg(theme::muted())
                .add_modifier(Modifier::ITALIC),
        ))];
    }
    let mut lines = Vec::new();
    let last = t.messages.len().saturating_sub(1);
    for (i, m) in t.messages.iter().enumerate() {
        if i > 0 {
            lines.push(Line::from(""));
        }
        let (label, style) = if m.role == "user" {
            ("you", Style::default().fg(theme::muted()))
        } else {
            (
                "model",
                Style::default()
                    .fg(theme::heat())
                    .add_modifier(Modifier::BOLD),
            )
        };
        lines.push(Line::from(Span::styled(label, style)));
        if m.content.is_empty() && m.role == "assistant" && streaming && i == last {
            lines.push(Line::from(Span::styled(
                "▌",
                Style::default().fg(theme::heat()),
            )));
            continue;
        }
        if m.content.is_empty() {
            continue;
        }
        for part in m.content.split('\n') {
            lines.push(Line::from(Span::styled(part.to_string(), theme::ink())));
        }
        if streaming && i == last && m.role == "assistant" {
            if let Some(last_line) = lines.last_mut() {
                last_line
                    .spans
                    .push(Span::styled("▌", Style::default().fg(theme::heat())));
            }
        }
    }
    lines
}

fn default_pick(catalog: &[CatalogEntry]) -> (Burner, String) {
    if let Some(e) = catalog.iter().find(|e| e.burner == Burner::Mlx) {
        return (e.burner, e.model.clone());
    }
    if let Some(e) = catalog.first() {
        return (e.burner, e.model.clone());
    }
    (Burner::Mlx, stove::default_mlx_model())
}

fn heat_label(burner: Burner) -> String {
    match stove::status_one(burner) {
        Ok(st) => format!("{} {}", burner.id(), st.heat().label()),
        Err(_) => burner.id().to_string(),
    }
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

fn model_label(model: &str) -> String {
    let name = model
        .trim_end_matches('/')
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(model);
    truncate(name, 40)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let take = max.saturating_sub(1);
        format!("{}…", s.chars().take(take).collect::<String>())
    }
}

fn trim_empty_assistant(t: &mut Transcript) {
    if t.messages
        .last()
        .is_some_and(|m| m.role == "assistant" && m.content.trim().is_empty())
    {
        t.messages.pop();
    }
}
