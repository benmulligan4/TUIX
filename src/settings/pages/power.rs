/// Power settings — shutdown/restart TUIX.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, cursor: usize, _scroll: usize) {
    let value_style = Style::default().fg(Color::White);

    let items: &[&str] = &[
        "Shut Down TUIX",
        "Restart TUIX",
    ];

    let mut lines: Vec<Line> = Vec::new();
    for (i, label) in items.iter().enumerate() {
        let prefix = if i == cursor { "  » " } else { "    " };
        let style = if i == cursor {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            value_style
        };
        lines.push(Line::from(Span::styled(format!("{}{}", prefix, label), style)));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

pub fn item_count() -> usize { 2 }

pub fn handle_enter(cursor: usize) -> crate::settings::page::SettingsAction {
    use crate::settings::page::SettingsAction;
    match cursor {
        0 => SettingsAction::Quit,    // Shut Down TUIX
        1 => SettingsAction::Restart, // Restart TUIX
        _ => SettingsAction::None,
    }
}
