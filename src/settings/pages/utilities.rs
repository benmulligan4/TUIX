/// Utility settings — on-screen keyboard toggle, clear logs.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::settings::persistence;

pub fn render(frame: &mut Frame, area: Rect, cursor: usize, _scroll: usize) {
    let settings = persistence::load();
    let osk_enabled = persistence::get_bool(&settings, "utilities.onscreen_keyboard_enabled", true);

    let items: Vec<(&str, String)> = vec![
        ("On-Screen Keyboard", if osk_enabled { "Enabled".into() } else { "Disabled".into() }),
        ("Clear Logs", "".into()),
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
        if value.is_empty() {
            lines.push(Line::from(Span::styled(format!("{}{}", prefix, label), style)));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("{}{:<24}", prefix, label), style),
                Span::styled(format!("  {}", value), style),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

pub fn item_count() -> usize { 2 }

pub fn handle_enter(cursor: usize) -> bool {
    match cursor {
        0 => {
            let mut settings = persistence::load();
            let current = persistence::get_bool(&settings, "utilities.onscreen_keyboard_enabled", true);
            persistence::set(&mut settings, "utilities.onscreen_keyboard_enabled", serde_json::Value::Bool(!current));
            persistence::save(&settings);
            false
        }
        1 => {
            // Clear logs — truncate tuix.log
            let _ = std::fs::write("tuix.log", "");
            true // Signal popup
        }
        _ => false,
    }
}
