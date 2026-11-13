//! Markdown to ratatui lines, styled with the chef palette.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme;

#[derive(Clone)]
struct WrapMode {
    first: Option<String>,
    cont: Option<String>,
    words: bool,
}

impl WrapMode {
    fn prose() -> Self {
        Self {
            first: None,
            cont: None,
            words: true,
        }
    }
}

struct ListCtx {
    next: Option<u64>,
}

struct TableBuild {
    rows: Vec<Vec<Vec<Span<'static>>>>,
    current_row: Vec<Vec<Span<'static>>>,
    has_header: bool,
}

struct Renderer {
    width: usize,
    lines: Vec<Line<'static>>,
    buf: Vec<Span<'static>>,
    style_stack: Vec<Style>,
    lists: Vec<ListCtx>,
    item_first: Option<String>,
    item_cont: Option<String>,
    quote: usize,
    in_code: bool,
    code: String,
    in_meta: bool,
    meta: String,
    link: Option<String>,
    table: Option<TableBuild>,
    in_table_head: bool,
}

/// Parse markdown and wrap to `width`. Every row is padded so leftover cells cannot ghost.
pub fn render(src: &str, width: usize) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
    let mut r = Renderer::new(width);
    for ev in Parser::new_ext(src, opts) {
        r.event(ev);
    }
    r.finish()
}

impl Renderer {
    fn new(width: usize) -> Self {
        Self {
            width,
            lines: Vec::new(),
            buf: Vec::new(),
            style_stack: Vec::new(),
            lists: Vec::new(),
            item_first: None,
            item_cont: None,
            quote: 0,
            in_code: false,
            code: String::new(),
            in_meta: false,
            meta: String::new(),
            link: None,
            table: None,
            in_table_head: false,
        }
    }

    fn style(&self) -> Style {
        let mut s = Style::default();
        for layer in &self.style_stack {
            s = s.patch(*layer);
        }
        s
    }

    fn event(&mut self, ev: Event<'_>) {
        match ev {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(t) => self.push(&t, self.text_style()),
            Event::Code(t) => self.push(&t, self.style().fg(theme::sage())),
            Event::SoftBreak => self.push(" ", self.style()),
            Event::HardBreak => self.push("\n", self.style()),
            Event::Rule => {
                self.flush(self.para_mode(), false);
                self.lines.push(rule_line(self.width));
            }
            Event::TaskListMarker(done) => {
                self.push(if done { "[x] " } else { "[ ] " }, self.style());
            }
            Event::Html(_) | Event::InlineHtml(_) | Event::FootnoteReference(_) => {}
            _ => {}
        }
    }

    fn text_style(&self) -> Style {
        if self.in_meta {
            Style::default().fg(theme::muted())
        } else if self.in_code {
            Style::default().fg(theme::sage())
        } else {
            self.style()
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {}
            Tag::Heading { level, .. } => {
                self.flush(self.para_mode(), false);
                self.style_stack.push(heading_style(level));
            }
            Tag::List(start) => {
                if !self.buf.is_empty() {
                    self.flush_item();
                }
                self.lists.push(ListCtx { next: start });
            }
            Tag::Item => {
                let marker = self.item_marker();
                let hang = marker.chars().count();
                self.item_first = Some(marker);
                self.item_cont = Some(" ".repeat(hang));
            }
            Tag::Emphasis => {
                self.style_stack
                    .push(Style::default().add_modifier(Modifier::ITALIC));
            }
            Tag::Strong => {
                self.style_stack
                    .push(Style::default().add_modifier(Modifier::BOLD));
            }
            Tag::Strikethrough => {
                self.style_stack
                    .push(Style::default().add_modifier(Modifier::CROSSED_OUT));
            }
            Tag::Link { dest_url, .. } => {
                self.link = Some(dest_url.to_string());
                self.style_stack.push(Style::default().fg(theme::heat()));
            }
            Tag::Image { dest_url, .. } => {
                self.link = Some(dest_url.to_string());
                self.style_stack.push(Style::default().fg(theme::muted()));
                self.push("[", self.style());
            }
            Tag::BlockQuote(_) => {
                self.flush(self.para_mode(), false);
                self.quote += 1;
                self.style_stack.push(
                    Style::default()
                        .fg(theme::muted())
                        .add_modifier(Modifier::ITALIC),
                );
            }
            Tag::CodeBlock(_) => {
                self.flush(self.para_mode(), false);
                self.in_code = true;
                self.code.clear();
            }
            Tag::MetadataBlock(_) => {
                self.in_meta = true;
                self.meta.clear();
            }
            Tag::Table(_) => {
                self.flush(self.para_mode(), false);
                self.table = Some(TableBuild {
                    rows: Vec::new(),
                    current_row: Vec::new(),
                    has_header: false,
                });
            }
            Tag::TableHead => {
                self.in_table_head = true;
                if let Some(t) = &mut self.table {
                    t.has_header = true;
                }
            }
            Tag::TableRow => {}
            Tag::TableCell => {
                self.buf.clear();
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                if self.table.is_none() {
                    self.flush(self.para_mode(), true);
                }
            }
            TagEnd::Heading(_) => {
                self.flush(WrapMode::prose(), false);
                self.style_stack.pop();
                self.blank();
            }
            TagEnd::Item => self.flush_item(),
            TagEnd::List(_) => {
                self.lists.pop();
                if self.lists.is_empty() {
                    self.blank();
                }
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => {
                self.style_stack.pop();
            }
            TagEnd::Link => {
                self.style_stack.pop();
                if let Some(href) = self.link.take() {
                    if !href.is_empty() && !href.starts_with('#') {
                        self.push(&format!(" ({href})"), Style::default().fg(theme::muted()));
                    }
                }
            }
            TagEnd::Image => {
                self.push("]", self.style());
                self.style_stack.pop();
                if let Some(href) = self.link.take() {
                    if !href.is_empty() {
                        self.push(&format!(" ({href})"), Style::default().fg(theme::muted()));
                    }
                }
            }
            TagEnd::BlockQuote(_) => {
                self.quote = self.quote.saturating_sub(1);
                self.style_stack.pop();
                self.blank();
            }
            TagEnd::CodeBlock => self.end_code(),
            TagEnd::MetadataBlock(_) => self.end_meta(),
            TagEnd::TableCell => self.end_table_cell(),
            TagEnd::TableHead => {
                self.flush_table_row();
                self.in_table_head = false;
            }
            TagEnd::TableRow => self.flush_table_row(),
            TagEnd::Table => {
                self.flush_table_row();
                self.emit_table();
            }
            _ => {}
        }
    }

    fn para_mode(&self) -> WrapMode {
        if self.quote == 0 {
            return WrapMode::prose();
        }
        let p = "│ ".repeat(self.quote);
        WrapMode {
            first: Some(p.clone()),
            cont: Some(p),
            words: true,
        }
    }

    fn item_marker(&mut self) -> String {
        let depth = self.lists.len().saturating_sub(1);
        let indent = "  ".repeat(depth);
        match self.lists.last_mut() {
            Some(ListCtx { next: Some(n) }) => {
                let m = format!("{indent}{n}. ");
                *n += 1;
                m
            }
            _ => format!("{indent}• "),
        }
    }

    fn flush_item(&mut self) {
        let mode = WrapMode {
            first: self.item_first.take(),
            cont: self.item_cont.take(),
            words: true,
        };
        self.flush(mode, false);
    }

    fn end_code(&mut self) {
        self.in_code = false;
        let style = Style::default().fg(theme::sage());
        let mode = WrapMode {
            first: Some(" ".into()),
            cont: Some(" ".into()),
            words: false,
        };
        let mut parts: Vec<&str> = self.code.split('\n').collect();
        if parts.last() == Some(&"") {
            parts.pop();
        }
        for line in parts {
            self.lines.extend(wrap_spans(
                vec![Span::styled(line.to_string(), style)],
                self.width,
                mode.clone(),
            ));
        }
        self.code.clear();
        self.blank();
    }

    fn end_meta(&mut self) {
        self.in_meta = false;
        let style = Style::default().fg(theme::muted());
        let text = std::mem::take(&mut self.meta);
        let mut parts: Vec<&str> = text.split('\n').collect();
        while parts.last() == Some(&"") {
            parts.pop();
        }
        while parts.first() == Some(&"") {
            parts.remove(0);
        }
        for line in parts {
            self.lines.extend(wrap_spans(
                vec![Span::styled(line.to_string(), style)],
                self.width,
                WrapMode::prose(),
            ));
        }
        if !self.lines.is_empty() {
            self.lines.push(rule_line(self.width));
        }
        self.blank();
    }

    fn push(&mut self, text: &str, style: Style) {
        if self.in_meta {
            self.meta.push_str(text);
            return;
        }
        if self.in_code {
            self.code.push_str(text);
            return;
        }
        for (i, piece) in text.split('\n').enumerate() {
            if i > 0 {
                self.buf.push(Span::raw("\n"));
            }
            if piece.is_empty() {
                continue;
            }
            if let Some(last) = self.buf.last_mut() {
                if last.style == style && last.content.as_ref() != "\n" {
                    last.content.to_mut().push_str(piece);
                    continue;
                }
            }
            self.buf.push(Span::styled(piece.to_string(), style));
        }
    }

    fn flush(&mut self, mode: WrapMode, blank_after: bool) {
        if self.buf.is_empty() && mode.first.is_none() {
            if blank_after {
                self.blank();
            }
            return;
        }
        let spans = std::mem::take(&mut self.buf);
        self.lines.extend(wrap_spans(spans, self.width, mode));
        if blank_after {
            self.blank();
        }
    }

    fn blank(&mut self) {
        if self.lines.last().map(is_blank).unwrap_or(true) {
            return;
        }
        self.lines.push(empty_row(self.width));
    }

    fn end_table_cell(&mut self) {
        let mut spans = std::mem::take(&mut self.buf);
        if self.in_table_head {
            spans = bold_spans(spans);
        }
        if let Some(t) = &mut self.table {
            t.current_row.push(spans);
        }
    }

    fn flush_table_row(&mut self) {
        if let Some(t) = &mut self.table {
            if !t.current_row.is_empty() {
                t.rows.push(std::mem::take(&mut t.current_row));
            }
        }
    }

    fn emit_table(&mut self) {
        let Some(table) = self.table.take() else {
            return;
        };
        self.in_table_head = false;
        if table.rows.is_empty() {
            return;
        }
        let n = table.rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if n == 0 {
            return;
        }
        let mut rows = table.rows;
        for row in &mut rows {
            row.resize(n, Vec::new());
        }
        let chrome = 3 * n + 1;
        let avail = self.width.saturating_sub(chrome);
        if avail < n {
            self.emit_table_stacked(rows, table.has_header);
            return;
        }
        let naturals: Vec<usize> = (0..n)
            .map(|c| rows.iter().map(|r| span_chars(&r[c])).max().unwrap_or(0))
            .collect();
        let widths = allocate_widths(&naturals, avail);
        if n >= 2 && widths.last().copied().unwrap_or(0) < 8 {
            self.emit_table_stacked(rows, table.has_header);
            return;
        }
        self.emit_table_grid(rows, widths);
        self.blank();
    }

    fn emit_table_grid(&mut self, rows: Vec<Vec<Vec<Span<'static>>>>, widths: Vec<usize>) {
        let border = Style::default().fg(theme::muted());
        self.lines
            .push(table_rule("┌", "┬", "┐", &widths, self.width, border));
        for (ri, row) in rows.into_iter().enumerate() {
            if ri > 0 {
                self.lines
                    .push(table_rule("├", "┼", "┤", &widths, self.width, border));
            }
            let wrapped: Vec<Vec<Vec<Span<'static>>>> = row
                .into_iter()
                .enumerate()
                .map(|(ci, cell)| {
                    wrap_spans(cell, widths[ci], WrapMode::prose())
                        .into_iter()
                        .map(|l| l.spans)
                        .collect()
                })
                .collect();
            let height = wrapped.iter().map(|c| c.len()).max().unwrap_or(1).max(1);
            for y in 0..height {
                let mut spans = vec![Span::styled("│ ", border)];
                for (ci, cell_lines) in wrapped.iter().enumerate() {
                    if ci > 0 {
                        spans.push(Span::styled(" │ ", border));
                    }
                    if let Some(line) = cell_lines.get(y) {
                        spans.extend(line.iter().cloned());
                    } else {
                        spans.push(Span::raw(" ".repeat(widths[ci])));
                    }
                }
                spans.push(Span::styled(" │", border));
                self.lines.push(pad_row(spans, self.width));
            }
        }
        self.lines
            .push(table_rule("└", "┴", "┘", &widths, self.width, border));
    }

    fn emit_table_stacked(&mut self, rows: Vec<Vec<Vec<Span<'static>>>>, has_header: bool) {
        let start = if has_header { 1 } else { 0 };
        for row in rows.into_iter().skip(start) {
            let mut cells = row.into_iter();
            let Some(label) = cells.next() else {
                continue;
            };
            let mut value = Vec::new();
            for (i, cell) in cells.enumerate() {
                if i > 0 {
                    value.push(Span::raw("  "));
                }
                value.extend(cell);
            }
            self.lines
                .extend(wrap_spans(label, self.width, WrapMode::prose()));
            if span_chars(&value) > 0 {
                self.lines.extend(wrap_spans(
                    value,
                    self.width,
                    WrapMode {
                        first: Some("  ".into()),
                        cont: Some("  ".into()),
                        words: true,
                    },
                ));
            }
            self.blank();
        }
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        if self.table.is_some() {
            if !self.buf.is_empty() {
                self.end_table_cell();
            }
            self.flush_table_row();
            self.emit_table();
        }
        if !self.buf.is_empty() {
            self.flush(WrapMode::prose(), false);
        }
        while self.lines.last().map(is_blank).unwrap_or(false) {
            self.lines.pop();
        }
        self.lines
    }
}

fn bold_spans(spans: Vec<Span<'static>>) -> Vec<Span<'static>> {
    spans
        .into_iter()
        .map(|s| Span::styled(s.content, s.style.add_modifier(Modifier::BOLD)))
        .collect()
}

fn allocate_widths(naturals: &[usize], avail: usize) -> Vec<usize> {
    let n = naturals.len();
    if n == 0 {
        return Vec::new();
    }
    let mut widths = vec![1usize; n];
    let used: usize = widths.iter().sum();
    if used > avail {
        let each = (avail / n).max(1);
        return vec![each; n];
    }
    let mut rest = avail - used;
    for (i, natural) in naturals.iter().enumerate() {
        let want = (*natural).max(1).saturating_sub(widths[i]);
        let give = want.min(rest);
        widths[i] += give;
        rest -= give;
    }
    if let Some(last) = widths.last_mut() {
        *last += rest;
    }
    widths
}

fn table_rule(
    left: &str,
    mid: &str,
    right: &str,
    widths: &[usize],
    width: usize,
    border: Style,
) -> Line<'static> {
    let mut s = String::from(left);
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            s.push_str(mid);
        }
        s.extend(std::iter::repeat_n('─', w + 2));
    }
    s.push_str(right);
    pad_row(vec![Span::styled(s, border)], width)
}

fn heading_style(level: HeadingLevel) -> Style {
    let s = Style::default().fg(theme::heat());
    match level {
        HeadingLevel::H1 | HeadingLevel::H2 => s.add_modifier(Modifier::BOLD),
        _ => s,
    }
}

fn rule_line(width: usize) -> Line<'static> {
    Line::from(Span::styled(
        "─".repeat(width),
        Style::default().fg(theme::muted()),
    ))
}

fn is_blank(line: &Line) -> bool {
    line.spans
        .iter()
        .all(|s| s.content.chars().all(|c| c == ' '))
}

fn empty_row(width: usize) -> Line<'static> {
    Line::from(Span::raw(" ".repeat(width)))
}

fn span_chars(spans: &[Span]) -> usize {
    spans.iter().map(|s| s.content.chars().count()).sum()
}

fn pad_row(mut spans: Vec<Span<'static>>, width: usize) -> Line<'static> {
    let used = span_chars(&spans);
    if used < width {
        spans.push(Span::raw(" ".repeat(width - used)));
    }
    Line::from(spans)
}

fn fit_prefix(p: Option<String>, width: usize) -> Option<String> {
    match p {
        Some(s) if s.chars().count() < width => Some(s),
        _ => None,
    }
}

fn tokens(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut last = 0;
    let mut prev: Option<bool> = None;
    for (i, ch) in s.char_indices() {
        let ws = ch.is_whitespace();
        if let Some(p) = prev {
            if p != ws {
                out.push(&s[last..i]);
                last = i;
            }
        }
        prev = Some(ws);
    }
    if last < s.len() {
        out.push(&s[last..]);
    }
    out
}

fn wrap_spans(spans: Vec<Span<'static>>, width: usize, mode: WrapMode) -> Vec<Line<'static>> {
    let mut w = Wrapper {
        width,
        first: fit_prefix(mode.first, width).unwrap_or_default(),
        cont: fit_prefix(mode.cont, width).unwrap_or_default(),
        rows: Vec::new(),
        cur: Vec::new(),
        col: 0,
        is_first: true,
        line_open: false,
    };
    if spans.is_empty() {
        w.ensure_line();
        return w.finish();
    }
    for span in spans {
        if span.content.as_ref() == "\n" {
            w.force_break();
            continue;
        }
        if mode.words {
            for tok in tokens(&span.content) {
                if tok.chars().all(char::is_whitespace) {
                    w.push_ws(tok, span.style);
                } else {
                    w.push_word(tok, span.style);
                }
            }
        } else {
            w.push_chars(&span.content, span.style);
        }
    }
    w.finish()
}

struct Wrapper {
    width: usize,
    first: String,
    cont: String,
    rows: Vec<Line<'static>>,
    cur: Vec<Span<'static>>,
    col: usize,
    is_first: bool,
    line_open: bool,
}

impl Wrapper {
    fn active_prefix(&self) -> &str {
        if self.is_first {
            &self.first
        } else {
            &self.cont
        }
    }

    fn prefix_len(&self) -> usize {
        self.active_prefix().chars().count()
    }

    fn ensure_line(&mut self) {
        if self.line_open {
            return;
        }
        let p = self.active_prefix().to_string();
        if p.is_empty() {
            self.cur = Vec::new();
            self.col = 0;
        } else {
            self.col = p.chars().count();
            self.cur = vec![Span::raw(p)];
        }
        self.line_open = true;
    }

    fn flush_line(&mut self) {
        if !self.line_open {
            return;
        }
        self.rows
            .push(pad_row(std::mem::take(&mut self.cur), self.width));
        self.col = 0;
        self.line_open = false;
        self.is_first = false;
    }

    fn force_break(&mut self) {
        if self.line_open {
            self.flush_line();
        } else {
            self.rows.push(empty_row(self.width));
            self.is_first = false;
        }
    }

    fn append(&mut self, text: &str, style: Style) {
        if text.is_empty() {
            return;
        }
        self.col += text.chars().count();
        if let Some(last) = self.cur.last_mut() {
            if last.style == style {
                last.content.to_mut().push_str(text);
                return;
            }
        }
        self.cur.push(Span::styled(text.to_string(), style));
    }

    fn push_ws(&mut self, tok: &str, style: Style) {
        if !self.line_open || self.col == self.prefix_len() {
            return;
        }
        let len = tok.chars().count();
        if self.col + len > self.width {
            self.flush_line();
            return;
        }
        self.append(tok, style);
    }

    fn push_word(&mut self, word: &str, style: Style) {
        self.ensure_line();
        let len = word.chars().count();
        let space = self.width.saturating_sub(self.col);
        if len <= space {
            self.append(word, style);
            return;
        }
        if self.col > self.prefix_len() {
            self.flush_line();
            self.ensure_line();
            let space = self.width.saturating_sub(self.col);
            if len <= space {
                self.append(word, style);
                return;
            }
        }
        self.push_long_token(word, style);
    }

    fn push_long_token(&mut self, mut rest: &str, style: Style) {
        while !rest.is_empty() {
            self.ensure_line();
            let space = self.width.saturating_sub(self.col);
            if space == 0 {
                self.flush_line();
                continue;
            }
            let rest_len = rest.chars().count();
            if rest_len <= space {
                self.append(rest, style);
                return;
            }
            let prefix: String = rest.chars().take(space).collect();
            let take = prefix
                .rfind('/')
                .filter(|&i| i + 1 >= 4)
                .map(|i| prefix[..=i].chars().count())
                .unwrap_or(space);
            let chunk: String = rest.chars().take(take).collect();
            let bytes = chunk.len();
            self.append(&chunk, style);
            rest = &rest[bytes..];
            if !rest.is_empty() {
                self.flush_line();
            }
        }
    }

    fn push_chars(&mut self, text: &str, style: Style) {
        for ch in text.chars() {
            self.ensure_line();
            if self.col >= self.width {
                self.flush_line();
                self.ensure_line();
            }
            self.append(&ch.to_string(), style);
        }
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        if self.line_open {
            self.flush_line();
        } else if self.rows.is_empty() {
            self.ensure_line();
            self.flush_line();
        }
        self.rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn trimmed(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|l| plain(l).trim_end().to_string())
            .collect()
    }

    #[test]
    fn heading_and_strong() {
        let lines = render("# Hi\n\nHello **there**.", 20);
        let t = trimmed(&lines);
        assert_eq!(t[0], "Hi");
        assert!(t.iter().any(|l| l.contains("Hello there.")));
        assert!(!t.iter().any(|l| l.contains("# Hi")));
    }

    #[test]
    fn frontmatter_then_body() {
        let md = "---\nname: x\n---\n\n# Title\n";
        let t = trimmed(&render(md, 40));
        assert!(t.iter().any(|l| l.contains("name: x")));
        assert!(t.iter().any(|l| l == "Title"));
    }

    #[test]
    fn list_and_code() {
        let md = "- one\n- two\n\n```\ncode\n```\n";
        let t = trimmed(&render(md, 40));
        assert!(t.iter().any(|l| l.contains("• one")));
        assert!(t.iter().any(|l| l.contains("code")));
    }

    #[test]
    fn wraps_words() {
        let t = trimmed(&render("hello world", 8));
        assert_eq!(t, vec!["hello", "world"]);
    }

    #[test]
    fn rows_padded_to_width() {
        let lines = render("hi", 10);
        for l in &lines {
            assert_eq!(plain(l).chars().count(), 10);
        }
    }

    #[test]
    fn skill_md_hides_heading_hashes() {
        let md = include_str!("../testdata/pantry/skills/kitchen/SKILL.md");
        let blob = trimmed(&render(md, 60)).join("\n");
        assert!(blob.contains("kitchen"));
        assert!(!blob.contains("# kitchen\n"));
        assert!(blob.contains("name: kitchen"));
    }

    #[test]
    fn table_renders_grid_not_pipes() {
        let md =
            "| Piece | Location |\n|-------|----------|\n| Model | `~/foo` |\n| Server | bar |\n";
        let t = trimmed(&render(md, 40));
        let blob = t.join("\n");
        assert!(blob.contains("Piece"));
        assert!(blob.contains("Location"));
        assert!(blob.contains("Model"));
        assert!(blob.contains("foo"));
        assert!(t.iter().any(|l| l.contains('│')));
        assert_eq!(t.iter().filter(|l| l.contains('┼')).count(), 2);
        assert!(!blob.contains("|-------"));
        assert!(!blob.contains("| Piece |"));
    }

    #[test]
    fn table_rows_padded_to_width() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n";
        for l in render(md, 24) {
            assert_eq!(plain(&l).chars().count(), 24);
        }
    }

    #[test]
    fn skill_table_not_raw_pipes() {
        let md = include_str!("../testdata/pantry/skills/chef-add-skill/SKILL.md");
        let t = trimmed(&render(md, 60));
        let blob = t.join("\n");
        assert!(blob.contains("Section"));
        assert!(blob.contains("Pitfalls"));
        assert!(t.iter().any(|l| l.contains('│')));
        assert!(!blob.contains("|-------"));
        assert!(!blob.contains("| Section |"));
    }
}
