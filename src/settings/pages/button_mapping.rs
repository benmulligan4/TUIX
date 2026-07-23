/// Button Mapping settings.

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
    let toggle_key = persistence::get_str(&settings, "button_mapping.focus_toggle_key", "Tab");

    let items: Vec<(&str, String)> = vec![
        ("Nav/Main Toggle Key", toggle_key),
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
            Span::styled(format!("{}{:<24}", prefix, label), style),
            Span::styled(format!("  {}", value), style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

pub fn item_count() -> usize { 1 }

pub fn handle_enter(_cursor: usize) {
    // Button remapping will require a "press a key" capture mode — placeholder
}
