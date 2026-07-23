/// Updates & Git settings — branch, pull, update, uninstall.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, cursor: usize, _scroll: usize) {
    let branch = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into());

    let items: Vec<&str> = vec![
        "Change Branch",
        "Pull & Rebuild",
        "Update Applications",
        "Update Dashboards",
        "Uninstall Application",
        "Uninstall Dashboard",
    ];

    let value_style = Style::default().fg(Color::White);
    let label_style = Style::default().fg(Color::DarkGray);
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("    Current Branch:  ", label_style),
        Span::styled(&branch, value_style),
    ]));
    lines.push(Line::from(""));

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

pub fn item_count() -> usize { 6 }

pub fn handle_enter(_cursor: usize) {
    // Git operations — will be implemented with subprocess execution
}
