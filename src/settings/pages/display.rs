/// Display settings — brightness, screen timeout, fullscreen.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::settings::persistence;

const TIMEOUT_OPTIONS: &[&str] = &["Never", "30 seconds", "1 minute", "2 minutes", "5 minutes", "10 minutes"];

pub fn render(frame: &mut Frame, area: Rect, cursor: usize, _scroll: usize, editing: bool) {
    let settings = persistence::load();
    let timeout = persistence::get_str(&settings, "display.screen_timeout", "Never");
    let fullscreen = persistence::get_bool(&settings, "display.fullscreen", false);

    let value_style = Style::default().fg(Color::White);

    let items: Vec<(&str, String)> = vec![
        ("Screen Timeout", timeout),
        ("Fullscreen (coming soon)", if fullscreen { "Enabled".into() } else { "Disabled".into() }),
    ];

    let mut lines: Vec<Line> = Vec::new();
    for (i, (label, value)) in items.iter().enumerate() {
        let is_active = i == cursor;
        let prefix = if is_active { "  » " } else { "    " };
        let style = if is_active {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            value_style
        };
        if is_active && editing && is_edit_mode_item(i) {
            lines.push(Line::from(vec![
                Span::styled(format!("{}{:<20}", prefix, label), style),
                Span::styled("  ◄ ", Style::default().fg(Color::Yellow)),
                Span::styled(value.as_str(), Style::default().fg(Color::White)),
                Span::styled(" ►", Style::default().fg(Color::Yellow)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("{}{:<20}", prefix, label), style),
                Span::styled(format!("  {}", value), style),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

pub fn is_edit_mode_item(cursor: usize) -> bool {
    cursor == 0 // Screen Timeout has 6 options
}

pub fn item_count() -> usize { 2 }

pub fn handle_cycle(cursor: usize, forward: bool) {
    if cursor != 0 { return; }
    let mut settings = persistence::load();
    let current = persistence::get_str(&settings, "display.screen_timeout", "Never");
    let idx = TIMEOUT_OPTIONS.iter().position(|&o| o == current).unwrap_or(0);
    let len = TIMEOUT_OPTIONS.len();
    let next_idx = if forward { (idx + 1) % len } else { (idx + len - 1) % len };
    let next = TIMEOUT_OPTIONS[next_idx];
    persistence::set(&mut settings, "display.screen_timeout", serde_json::Value::String(next.to_string()));
    persistence::save(&settings);
    crate::utilities::logging::settings(&format!("Screen timeout set to {}", next));
}

pub fn handle_enter(cursor: usize) {
    let mut settings = persistence::load();
    match cursor {
        0 => {
            // Cycle screen timeout
            let current = persistence::get_str(&settings, "display.screen_timeout", "Never");
            let idx = TIMEOUT_OPTIONS.iter().position(|&o| o == current).unwrap_or(0);
            let next = TIMEOUT_OPTIONS[(idx + 1) % TIMEOUT_OPTIONS.len()];
            persistence::set(&mut settings, "display.screen_timeout", serde_json::Value::String(next.to_string()));
            persistence::save(&settings);
            crate::utilities::logging::settings(&format!("Screen timeout set to {}", next));
        }
        1 => {
            // Toggle fullscreen
            let current = persistence::get_bool(&settings, "display.fullscreen", false);
            persistence::set(&mut settings, "display.fullscreen", serde_json::Value::Bool(!current));
            persistence::save(&settings);
            crate::utilities::logging::settings(&format!("Fullscreen {}", if !current { "enabled" } else { "disabled" }));
        }
        _ => {}
    }
}
