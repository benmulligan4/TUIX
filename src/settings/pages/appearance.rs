/// Appearance settings — clock, default dashboard, accent colour.

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

pub fn render(frame: &mut Frame, area: Rect, cursor: usize, _scroll: usize) {
    let settings = persistence::load();
    let accent = persistence::get_str(&settings, "appearance.accent_color", "Cyan");
    let clock_on = persistence::get_bool(&settings, "appearance.clock_enabled", false);
    let clock_24h = persistence::get_bool(&settings, "appearance.clock_format_24h", true);
    let clock_secs = persistence::get_bool(&settings, "appearance.clock_show_seconds", false);
    let font = persistence::get_str(&settings, "appearance.font", "Default");
    let default_dash = persistence::get_str(&settings, "default_dashboard", "Dashboard-1");

    let items: Vec<(&str, String)> = vec![
        ("Accent Colour", accent),
        ("Clock", if clock_on { "Enabled".into() } else { "Disabled".into() }),
        ("Clock Format", if clock_24h { "24 hour".into() } else { "12 hour".into() }),
        ("Show Seconds", if clock_secs { "Yes".into() } else { "No".into() }),
        ("Font", font),
        ("Default Dashboard", default_dash),
    ];

    let value_style = Style::default().fg(Color::White);
    let mut lines: Vec<Line> = Vec::new();
    for (i, (label, value)) in items.iter().enumerate() {
        let prefix = if i == cursor { "  » " } else { "    " };
        let style = if i == cursor {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            value_style
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{}{:<22}", prefix, label), style),
            Span::styled(format!("  {}", value), style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

pub fn item_count() -> usize { 6 }

pub fn handle_enter(cursor: usize) {
    let mut settings = persistence::load();
    match cursor {
        0 => {
            let current = persistence::get_str(&settings, "appearance.accent_color", "Cyan");
            let idx = ACCENT_COLORS.iter().position(|&c| c == current).unwrap_or(0);
            let next = ACCENT_COLORS[(idx + 1) % ACCENT_COLORS.len()];
            persistence::set(&mut settings, "appearance.accent_color", serde_json::Value::String(next.to_string()));
        }
        1 => {
            let current = persistence::get_bool(&settings, "appearance.clock_enabled", false);
            persistence::set(&mut settings, "appearance.clock_enabled", serde_json::Value::Bool(!current));
        }
        2 => {
            let current = persistence::get_bool(&settings, "appearance.clock_format_24h", true);
            persistence::set(&mut settings, "appearance.clock_format_24h", serde_json::Value::Bool(!current));
        }
        3 => {
            let current = persistence::get_bool(&settings, "appearance.clock_show_seconds", false);
            persistence::set(&mut settings, "appearance.clock_show_seconds", serde_json::Value::Bool(!current));
        }
        4 => {
            let current = persistence::get_str(&settings, "appearance.font", "Default");
            let idx = FONT_OPTIONS.iter().position(|&f| f == current).unwrap_or(0);
            let next = FONT_OPTIONS[(idx + 1) % FONT_OPTIONS.len()];
            persistence::set(&mut settings, "appearance.font", serde_json::Value::String(next.to_string()));
        }
        5 => {
            let dash_names = persistence::load_dashboard_names();
            if !dash_names.is_empty() {
                let current = persistence::get_str(&settings, "default_dashboard", "Dashboard-1");
                let idx = dash_names.iter().position(|d| d == &current).unwrap_or(0);
                let next = &dash_names[(idx + 1) % dash_names.len()];
                persistence::set(&mut settings, "default_dashboard", serde_json::Value::String(next.clone()));
            }
        }
        _ => {}
    }
    persistence::save(&settings);
}
