/// Display settings — fullscreen.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::settings::persistence;

pub fn render(frame: &mut Frame, area: Rect, cursor: usize, _scroll: usize, _editing: bool) {
    let settings = persistence::load();
    let fullscreen = persistence::get_bool(&settings, "display.fullscreen", false);

    let value_style = Style::default().fg(Color::White);

    let items: Vec<(&str, String)> = vec![
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
        lines.push(Line::from(vec![
            Span::styled(format!("{}{:<20}", prefix, label), style),
            Span::styled(format!("  {}", value), style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

pub fn is_edit_mode_item(_cursor: usize) -> bool { false }

pub fn item_count() -> usize { 1 }

pub fn handle_cycle(_cursor: usize, _forward: bool) {}

pub fn handle_enter(cursor: usize) {
    if cursor != 0 { return; }
    let mut settings = persistence::load();
    let current = persistence::get_bool(&settings, "display.fullscreen", false);
    persistence::set(&mut settings, "display.fullscreen", serde_json::Value::Bool(!current));
    persistence::save(&settings);
    crate::utilities::logging::settings(&format!("Fullscreen {}", if !current { "enabled" } else { "disabled" }));
}
