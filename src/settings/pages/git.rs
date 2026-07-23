/// Scripts & Git settings — run scripts, pull, rebuild.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
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

    let script_ext = if cfg!(target_os = "windows") { ".bat" } else { ".sh" };

    let items: Vec<String> = vec![
        "Git Pull".to_string(),
        "Git Pull & Rebuild".to_string(),
        format!("Run Build Script (build{})", script_ext),
        format!("Run Install Script (install{})", script_ext),
        format!("Run Clone Script (clone{})", script_ext),
        format!("Run Setup Script (setup{})", script_ext),
    ];

    let value_style = Style::default().fg(Color::White);
    let label_style = Style::default().fg(Color::DarkGray);

    // Split area: top for items, bottom for terminal output
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length((items.len() as u16) + 4), Constraint::Fill(1)])
        .split(area);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("    Current Branch:  ", label_style),
        Span::styled(&branch, value_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("    Script Type:     ", label_style),
        Span::styled(
            if cfg!(target_os = "windows") { "Windows (.bat)" } else { "Linux (.sh)" },
            value_style,
        ),
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

    frame.render_widget(Paragraph::new(lines), sections[0]);

    // Terminal output area (read-only)
    let term_lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "  ─── Terminal Output ───",
            label_style,
        )),
        Line::from(Span::styled(
            "  (Run a script to see output here)",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    frame.render_widget(Paragraph::new(term_lines), sections[1]);
}

pub fn item_count() -> usize { 6 }

pub fn handle_enter(_cursor: usize) {
    // Script/git execution — subprocess output capture to be implemented
}
