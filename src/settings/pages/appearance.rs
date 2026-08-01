/// Appearance settings — clock, default dashboard, TUIX colour.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{BorderType, Paragraph},
    Frame,
};

use crate::settings::persistence;

const ACCENT_COLORS: &[&str] = &[
    "Cyan", "Green", "Yellow", "Blue", "Magenta", "Red", "White",
    "Light Blue", "Orange", "Pink", "Teal", "Purple", "Grey", "Rainbow",
];

const BORDER_STYLES: &[&str] = &["Rounded", "Single", "Double", "None"];

/// Convert a border style name to a ratatui BorderType.
pub fn border_type_from_name(name: &str) -> BorderType {
    match name {
        "Double" => BorderType::Double,
        "Rounded" => BorderType::Rounded,
        "None" => BorderType::Plain,
        _ => BorderType::Plain, // "Single" and fallback
    }
}

/// Convert a colour name string to a ratatui Color.
/// "Rainbow" animates through the spectrum on every call.
pub fn color_from_name(name: &str) -> Color {
    match name {
        "Cyan"       => Color::Cyan,
        "Green"      => Color::Green,
        "Yellow"     => Color::Yellow,
        "Blue"       => Color::Blue,
        "Magenta"    => Color::Magenta,
        "Red"        => Color::Red,
        "White"      => Color::White,
        "Light Blue" => Color::Rgb(135, 206, 235),
        "Orange"     => Color::Rgb(255, 140, 0),
        "Pink"       => Color::Rgb(255, 105, 180),
        "Teal"       => Color::Rgb(0, 178, 178),
        "Purple"     => Color::Rgb(148, 0, 211),
        "Grey"       => Color::Rgb(160, 160, 160),
        "Rainbow" => {
            // Animate through hues based on current time (changes ~10x per second)
            let ms = chrono::Local::now().timestamp_millis() as u64;
            let hue = (ms / 80) % 360;
            let (r, g, b) = hue_to_rgb(hue as f32);
            Color::Rgb(r, g, b)
        }
        _ => Color::Cyan,
    }
}

/// Convert a HSV hue (0-360, s=1, v=1) to an RGB triple.
fn hue_to_rgb(hue: f32) -> (u8, u8, u8) {
    let h = hue / 60.0;
    let i = h as u32 % 6;
    let f = h - h.floor();
    let q = ((1.0 - f) * 255.0) as u8;
    let t = (f * 255.0) as u8;
    match i {
        0 => (255, t,   0),
        1 => (q,   255, 0),
        2 => (0,   255, t),
        3 => (0,   q,   255),
        4 => (t,   0,   255),
        _ => (255, 0,   q),
    }
}

pub fn render(frame: &mut Frame, area: Rect, cursor: usize, _scroll: usize, editing: bool) {
    let settings = persistence::load();
    let accent = persistence::get_str(&settings, "appearance.accent_color", "Cyan");
    let clock_on = persistence::get_bool(&settings, "appearance.clock_enabled", false);
    let clock_24h = persistence::get_bool(&settings, "appearance.clock_format_24h", true);
    let clock_secs = persistence::get_bool(&settings, "appearance.clock_show_seconds", false);
    let default_dash = persistence::get_str(&settings, "default_dashboard", "Dashboard-1");
    let border_style = persistence::get_str(&settings, "appearance.border_style", "Rounded");
    let status_bar = persistence::get_bool(&settings, "appearance.status_bar_enabled", false);
    let boot_anim = persistence::get_bool(&settings, "appearance.boot_animation", true);
    let boot_anim_color = persistence::get_str(&settings, "appearance.boot_animation_color", "White");

    let items: Vec<(&str, String)> = vec![
        ("TUIX Colour", accent),
        ("Clock", if clock_on { "Enabled".into() } else { "Disabled".into() }),
        ("Clock Format", if clock_24h { "24 hour".into() } else { "12 hour".into() }),
        ("Show Seconds", if clock_secs { "Yes".into() } else { "No".into() }),
        ("Default Dashboard", default_dash),
        ("Border Style", border_style),
        ("Status Bar", if status_bar { "Enabled".into() } else { "Disabled".into() }),
        ("Boot Animation", if boot_anim { "Enabled".into() } else { "Disabled".into() }),
        ("Boot Animation Colour", boot_anim_color),
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
    matches!(cursor, 0 | 4 | 5 | 8) // TUIX Colour, Default Dashboard, Border Style, Boot Animation Colour
}

pub fn item_count() -> usize { 9 }

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
        5 => {
            let current = persistence::get_str(&settings, "appearance.border_style", "Rounded");
            let idx = BORDER_STYLES.iter().position(|&b| b == current).unwrap_or(0);
            let len = BORDER_STYLES.len();
            let next_idx = if forward { (idx + 1) % len } else { (idx + len - 1) % len };
            let next = BORDER_STYLES[next_idx];
            persistence::set(&mut settings, "appearance.border_style", serde_json::Value::String(next.to_string()));
            crate::utilities::logging::settings(&format!("Border style changed to {}", next));
        }
        8 => {
            let current = persistence::get_str(&settings, "appearance.boot_animation_color", "White");
            let idx = ACCENT_COLORS.iter().position(|&c| c == current).unwrap_or(0);
            let len = ACCENT_COLORS.len();
            let next_idx = if forward { (idx + 1) % len } else { (idx + len - 1) % len };
            let next = ACCENT_COLORS[next_idx];
            persistence::set(&mut settings, "appearance.boot_animation_color", serde_json::Value::String(next.to_string()));
            crate::utilities::logging::settings(&format!("Boot Animation Colour changed to {}", next));
        }
        _ => {}
    }
    persistence::save(&settings);
}

/// Handle Enter for binary-toggle items.
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
        6 => {
            let current = persistence::get_bool(&settings, "appearance.status_bar_enabled", false);
            persistence::set(&mut settings, "appearance.status_bar_enabled", serde_json::Value::Bool(!current));
            crate::utilities::logging::settings(&format!("Status bar {}", if !current { "enabled" } else { "disabled" }));
        }
        7 => {
            let current = persistence::get_bool(&settings, "appearance.boot_animation", true);
            persistence::set(&mut settings, "appearance.boot_animation", serde_json::Value::Bool(!current));
            crate::utilities::logging::settings(&format!("Boot Animation {}", if !current { "enabled" } else { "disabled" }));
        }
        _ => {}
    }
    persistence::save(&settings);
}