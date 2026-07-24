/// Appearance settings — clock, default dashboard, TUIX colour.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::settings::persistence;

const ACCENT_COLORS: &[&str] = &["Cyan", "Green", "Yellow", "Blue", "Magenta", "Red", "White"];
const FONT_OPTIONS: &[&str] = &["Default", "Monospace", "Serif", "Sans", "Narrow"];

/// Convert a colour name string to a ratatui Color.
pub fn color_from_name(name: &str) -> Color {
    match name {
        "Cyan" => Color::Cyan,
        "Green" => Color::Green,
        "Yellow" => Color::Yellow,
        "Blue" => Color::Blue,
        "Magenta" => Color::Magenta,
        "Red" => Color::Red,
        "White" => Color::White,
        _ => Color::Cyan,
    }
}

pub fn render(frame: &mut Frame, area: Rect, cursor: usize, _scroll: usize, editing: bool) {
    let settings = persistence::load();
    let accent = persistence::get_str(&settings, "appearance.accent_color", "Cyan");
    let clock_on = persistence::get_bool(&settings, "appearance.clock_enabled", false);
    let clock_24h = persistence::get_bool(&settings, "appearance.clock_format_24h", true);
    let clock_secs = persistence::get_bool(&settings, "appearance.clock_show_seconds", false);
    let font = persistence::get_str(&settings, "appearance.font", "Default");
    let default_dash = persistence::get_str(&settings, "default_dashboard", "Dashboard-1");

    let items: Vec<(&str, String)> = vec![
        ("TUIX Colour", accent),
        ("Clock", if clock_on { "Enabled".into() } else { "Disabled".into() }),
        ("Clock Format", if clock_24h { "24 hour".into() } else { "12 hour".into() }),
        ("Show Seconds", if clock_secs { "Yes".into() } else { "No".into() }),
        ("Font", font),
        ("Default Dashboard", default_dash),
    ];

    let value_style = Style::default().fg(Color::White);
    let mut lines: Vec<Line> = Vec::new();
    for (i, (label, value)) in items.iter().enumerate() {
        let is_active = i == cursor;
        let prefix = if is_active { "  » " } else { "    " };
        let style = if is_active {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            value_style
        };
        // For multi-option items in edit mode, show ◄ value ►
        if is_active && editing && is_edit_mode_item(i) {
            lines.push(Line::from(vec![
                Span::styled(format!("{}{:<22}", prefix, label), style),
                Span::styled("  ◄ ", Style::default().fg(Color::Yellow)),
                Span::styled(value.as_str(), Style::default().fg(Color::White)),
                Span::styled(" ►", Style::default().fg(Color::Yellow)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("{}{:<22}", prefix, label), style),
                Span::styled(format!("  {}", value), style),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

/// Returns true for items that use left/right cycling (more than 2 options).
pub fn is_edit_mode_item(cursor: usize) -> bool {
    matches!(cursor, 0 | 4 | 5) // TUIX Colour, Font, Default Dashboard
}

pub fn item_count() -> usize { 6 }

/// Cycle a multi-option item forward (+1) or backward (-1).
pub fn handle_cycle(cursor: usize, forward: bool) {
    let mut settings = persistence::load();
    match cursor {
        0 => {
            let current = persistence::get_str(&settings, "appearance.accent_color", "Cyan");
            let idx = ACCENT_COLORS.iter().position(|&c| c == current).unwrap_or(0);
            let len = ACCENT_COLORS.len();
            let next_idx = if forward { (idx + 1) % len } else { (idx + len - 1) % len };
            let next = ACCENT_COLORS[next_idx];
            persistence::set(&mut settings, "appearance.accent_color", serde_json::Value::String(next.to_string()));
            crate::utilities::logging::settings(&format!("TUIX Colour changed to {}", next));
        }
        4 => {
            let current = persistence::get_str(&settings, "appearance.font", "Default");
            let idx = FONT_OPTIONS.iter().position(|&f| f == current).unwrap_or(0);
            let len = FONT_OPTIONS.len();
            let next_idx = if forward { (idx + 1) % len } else { (idx + len - 1) % len };
            let next = FONT_OPTIONS[next_idx];
            persistence::set(&mut settings, "appearance.font", serde_json::Value::String(next.to_string()));
            crate::utilities::logging::settings(&format!("Font changed to {}", next));
        }
        5 => {
            let dash_names = persistence::load_dashboard_names();
            if !dash_names.is_empty() {
                let current = persistence::get_str(&settings, "default_dashboard", "Dashboard-1");
                let idx = dash_names.iter().position(|d| d == &current).unwrap_or(0);
                let len = dash_names.len();
                let next_idx = if forward { (idx + 1) % len } else { (idx + len - 1) % len };
                let next = &dash_names[next_idx];
                persistence::set(&mut settings, "default_dashboard", serde_json::Value::String(next.clone()));
                crate::utilities::logging::settings(&format!("Default dashboard changed to {}", next));
            }
        }
        _ => {}
    }
    persistence::save(&settings);
}

/// Handle Enter for binary-toggle items (Clock, Clock Format, Show Seconds).
pub fn handle_enter(cursor: usize) {
    let mut settings = persistence::load();
    match cursor {
        1 => {
            let current = persistence::get_bool(&settings, "appearance.clock_enabled", false);
            persistence::set(&mut settings, "appearance.clock_enabled", serde_json::Value::Bool(!current));
            crate::utilities::logging::settings(&format!("Clock {}", if !current { "enabled" } else { "disabled" }));
        }
        2 => {
            let current = persistence::get_bool(&settings, "appearance.clock_format_24h", true);
            persistence::set(&mut settings, "appearance.clock_format_24h", serde_json::Value::Bool(!current));
            crate::utilities::logging::settings(&format!("Clock format set to {}", if !current { "24h" } else { "12h" }));
        }
        3 => {
            let current = persistence::get_bool(&settings, "appearance.clock_show_seconds", false);
            persistence::set(&mut settings, "appearance.clock_show_seconds", serde_json::Value::Bool(!current));
            crate::utilities::logging::settings(&format!("Clock seconds {}", if !current { "shown" } else { "hidden" }));
        }
        _ => {}
    }
    persistence::save(&settings);
}