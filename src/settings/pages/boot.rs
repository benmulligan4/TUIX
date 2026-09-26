/// Boot animation settings — opened from the Appearance category.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use serde_json::Value;

use crate::settings::persistence;

use super::cycle_option;

const STYLES: &[&str] = &["Modern", "Retro"];
const COLOURS: &[&str] = &[
    "Default",
    "Rainbow Dynamic",
    "Rainbow Static",
    "Rainbow Vertical",
    "Nebula",
    "Aurora",
    "Warm",
    "Cool",
    "Neon",
    "Synthwave",
    "Sunset",
    "Ocean",
    "Matrix",
    "Ember",
];
const LOGOS: &[&str] = &["Static", "Faded"];
const DIRECTIONS: &[&str] = &["Center", "Up", "Down", "Left", "Right"];

/// Rows in display order. Colour, logo and direction only apply to the modern
/// intro, so they are hidden while Retro is selected.
#[derive(Clone, Copy, PartialEq)]
enum Row {
    Enabled,
    Style,
    Colour,
    Logo,
    Direction,
}

impl Row {
    /// True for rows picked from a fixed list with ◄ ►.
    fn cycles(self) -> bool {
        self != Row::Enabled
    }
}

fn rows(settings: &Value) -> Vec<Row> {
    let mut rows = vec![Row::Enabled, Row::Style];
    if style(settings) == "Modern" {
        rows.extend([Row::Colour, Row::Logo, Row::Direction]);
    }
    rows
}

fn row_at(settings: &Value, cursor: usize) -> Option<Row> {
    rows(settings).get(cursor).copied()
}

/// Animation style. Rainbow values used to live on this key, so anything that
/// is not Retro reads as Modern.
pub fn style(settings: &Value) -> String {
    let stored = persistence::get_str(settings, "appearance.boot_animation_style", "Modern");
    if stored.eq_ignore_ascii_case("Retro") { "Retro".into() } else { "Modern".into() }
}

pub fn colour(settings: &Value) -> String {
    let stored = persistence::get_str(settings, "appearance.boot_animation_colour", "");
    if COLOURS.contains(&stored.as_str()) {
        return stored;
    }
    let legacy = persistence::get_str(settings, "appearance.boot_animation_style", "");
    if COLOURS.contains(&legacy.as_str()) { legacy } else { "Default".into() }
}

pub fn logo(settings: &Value) -> String {
    let stored = persistence::get_str(settings, "appearance.boot_logo", "Static");
    if LOGOS.contains(&stored.as_str()) { stored } else { "Static".into() }
}

pub fn direction(settings: &Value) -> String {
    let stored = persistence::get_str(settings, "appearance.boot_reveal", "Center");
    if DIRECTIONS.contains(&stored.as_str()) { stored } else { "Center".into() }
}

fn label_value(row: Row, s: &Value) -> (&'static str, String) {
    match row {
        Row::Enabled => (
            "Intro Animation",
            if persistence::get_bool(s, "appearance.intro_animation_enabled", true) {
                "Enabled".into()
            } else {
                "Disabled".into()
            },
        ),
        Row::Style => ("Animation Style", style(s)),
        Row::Colour => ("Colour", colour(s)),
        Row::Logo => ("Logo", logo(s)),
        Row::Direction => ("Direction", direction(s)),
    }
}

pub fn render(frame: &mut Frame, area: Rect, cursor: usize, _scroll: usize, editing: bool) {
    let settings = persistence::load();

    let value_style = Style::default().fg(Color::White);
    let mut lines: Vec<Line> = Vec::new();
    for (i, row) in rows(&settings).iter().enumerate() {
        let (label, value) = label_value(*row, &settings);
        let is_active = i == cursor;
        let prefix = if is_active { "  » " } else { "    " };
        let style = if is_active {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            value_style
        };
        if is_active && editing && row.cycles() {
            lines.push(Line::from(vec![
                Span::styled(format!("{}{:<22}", prefix, label), style),
                Span::styled("  ◄ ", Style::default().fg(Color::Yellow)),
                Span::styled(value, Style::default().fg(Color::White)),
                Span::styled(" ►", Style::default().fg(Color::Yellow)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("{}{:<22}", prefix, label), style),
                Span::styled(format!("  {}", value), style),
            ]));
        }
    }

    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "    Back returns to Appearance",
        Style::default().fg(Color::DarkGray),
    )));

    frame.render_widget(Paragraph::new(lines), area);
}

/// Returns true for items that use left/right cycling.
pub fn is_edit_mode_item(cursor: usize) -> bool {
    row_at(&persistence::load(), cursor).map_or(false, |row| row.cycles())
}

pub fn item_count() -> usize {
    rows(&persistence::load()).len()
}

/// Cycle a multi-option item forward (+1) or backward (-1).
pub fn handle_cycle(cursor: usize, forward: bool) {
    let mut settings = persistence::load();
    let row = match row_at(&settings, cursor) {
        Some(row) => row,
        None => return,
    };
    match row {
        Row::Style => {
            let current = style(&settings);
            let next = cycle_option(&mut settings, "appearance.boot_animation_style", STYLES, &current, forward);
            crate::utilities::logging::settings(&format!("Boot animation set to {}", next));
        }
        Row::Colour => {
            let current = colour(&settings);
            let next = cycle_option(&mut settings, "appearance.boot_animation_colour", COLOURS, &current, forward);
            crate::utilities::logging::settings(&format!("Boot colour set to {}", next));
        }
        Row::Logo => {
            let current = logo(&settings);
            let next = cycle_option(&mut settings, "appearance.boot_logo", LOGOS, &current, forward);
            crate::utilities::logging::settings(&format!("Boot logo set to {}", next));
        }
        Row::Direction => {
            let current = direction(&settings);
            let next = cycle_option(&mut settings, "appearance.boot_reveal", DIRECTIONS, &current, forward);
            crate::utilities::logging::settings(&format!("Boot direction set to {}", next));
        }
        Row::Enabled => {}
    }
    persistence::save(&settings);
}

pub fn handle_enter(cursor: usize) {
    let mut settings = persistence::load();
    if row_at(&settings, cursor) != Some(Row::Enabled) {
        return;
    }
    let current = persistence::get_bool(&settings, "appearance.intro_animation_enabled", true);
    persistence::set(
        &mut settings,
        "appearance.intro_animation_enabled",
        serde_json::Value::Bool(!current),
    );
    crate::utilities::logging::settings(&format!(
        "Intro animation {}",
        if !current { "enabled" } else { "disabled" }
    ));
    persistence::save(&settings);
}
