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

pub fn render(frame: &mut Frame, area: Rect, cursor: usize, _scroll: usize) {
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
        let prefix = if i == cursor { "  » " } else { "    " };
        let style = if i == cursor {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            value_style
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{}{:<20}", prefix, label), style),
            Span::styled(format!("  {}", value), style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

pub fn item_count() -> usize { 2 }

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
