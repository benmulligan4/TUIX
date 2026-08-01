/// Scripts settings — run build/install/clone/setup scripts.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, cursor: usize, _scroll: usize) {
    let script_ext = if cfg!(target_os = "windows") { ".bat" } else { ".sh" };

    let label_style = Style::default().fg(Color::DarkGray);
    let value_style = Style::default().fg(Color::White);
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("    Script Type:  ", label_style),
        Span::styled(
            if cfg!(target_os = "windows") { "Windows (.bat)" } else { "Linux (.sh)" },
            value_style,
        ),
    ]));
    lines.push(Line::from(""));

    let items: Vec<String> = vec![
        format!("Run Build Script (build{})", script_ext),
        format!("Run Install Script (install{})", script_ext),
        format!("Run Clone Script (clone{})", script_ext),
        format!("Run Setup Script (setup{})", script_ext),
        "Run Git Pull".to_string(),
    ];

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

pub fn item_count() -> usize { 5 }

pub fn handle_enter(_cursor: usize) {
    // Script execution — will be implemented with subprocess + output capture
}
