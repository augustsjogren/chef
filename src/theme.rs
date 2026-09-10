//! Terminal-aware palette. Primary text follows the terminal default so light and dark themes both work.

use std::sync::OnceLock;

use ratatui::style::{Color, Modifier, Style};

static LIGHT_BG: OnceLock<bool> = OnceLock::new();

fn light_background() -> bool {
    *LIGHT_BG.get_or_init(|| {
        match std::env::var("CHEF_THEME")
            .ok()
            .as_deref()
            .map(str::to_ascii_lowercase)
        {
            Some(v) if v == "light" => return true,
            Some(v) if v == "dark" => return false,
            _ => {}
        }
        if let Ok(v) = std::env::var("COLORFGBG") {
            if let Some(bg) = v.rsplit(';').next() {
                if let Ok(n) = bg.parse::<u8>() {
                    return n >= 7;
                }
            }
        }
        false
    })
}

pub fn ink() -> Style {
    Style::default()
}

pub fn ink_bold() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

pub fn highlight() -> Style {
    Style::default().add_modifier(Modifier::REVERSED)
}

pub fn muted() -> Color {
    if light_background() {
        Color::Rgb(96, 92, 86)
    } else {
        Color::Rgb(120, 118, 112)
    }
}

pub fn heat() -> Color {
    if light_background() {
        Color::Rgb(180, 90, 16)
    } else {
        Color::Rgb(232, 140, 48)
    }
}

pub fn sage() -> Color {
    if light_background() {
        Color::Rgb(72, 118, 58)
    } else {
        Color::Rgb(138, 168, 122)
    }
}

pub fn charred() -> Color {
    if light_background() {
        Color::Rgb(168, 52, 36)
    } else {
        Color::Rgb(196, 72, 48)
    }
}
