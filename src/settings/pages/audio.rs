/// Audio settings — Raspberry Pi only.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, _cursor: usize, _scroll: usize) {
    if !cfg!(target_os = "linux") {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  This feature is available on Raspberry Pi only.",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        frame.render_widget(Paragraph::new(lines), area);
        return;
    }

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Audio settings — coming soon.",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}

pub fn item_count() -> usize { 0 }
