/// Settings page — rendered in the TUIX main container.
///
/// Implements the TUIX page API:
///     render(frame, area)  — draw the page inside the given area

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, border_style: Style) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Settings ")
        .title_alignment(ratatui::layout::Alignment::Right)
        .style(border_style);
    frame.render_widget(block, area);

    let inner = area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });

    let lines = vec![
        "",
        "  Settings",
        "  ─────────────────────────────────────",
        "",
        "  This page is under construction.",
        "",
        "  Press  Q  to return.",
    ];
    let content = lines.join("\n");
    let paragraph = Paragraph::new(Text::raw(content))
        .style(Style::default().fg(Color::White));
    frame.render_widget(paragraph, inner);
}
