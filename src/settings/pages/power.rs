/// Power settings — shutdown/restart TUIX, shutdown Raspberry Pi.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, cursor: usize, _scroll: usize) {
    let value_style = Style::default().fg(Color::White);
    let grey_style = Style::default().fg(Color::DarkGray);

    let mut lines: Vec<Line> = Vec::new();

    let items: &[&str] = &[
        "Shut Down TUIX",
        "Restart TUIX",
        "Shut Down Raspberry Pi",
    ];

    for (i, label) in items.iter().enumerate() {
        let is_pi_shutdown = i == 2;
        let available = !is_pi_shutdown || cfg!(target_os = "linux");

        let prefix = if i == cursor && available { "  » " } else { "    " };
        let style = if !available {
            grey_style
        } else if i == cursor {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            value_style
        };

        let suffix = if is_pi_shutdown && !available { "  (Raspberry Pi only)" } else { "" };
        lines.push(Line::from(Span::styled(
            format!("{}{}{}", prefix, label, suffix),
            style,
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

pub fn item_count() -> usize { 3 }

pub fn handle_enter(cursor: usize) -> crate::settings::page::SettingsAction {
    use crate::settings::page::SettingsAction;
    match cursor {
        0 => SettingsAction::Quit,    // Shut Down TUIX
        1 => SettingsAction::Restart, // Restart TUIX
        2 => {
            // Shut Down Raspberry Pi
            if cfg!(target_os = "linux") {
                let _ = std::process::Command::new("sudo")
                    .args(["shutdown", "-h", "now"])
                    .status();
            }
            SettingsAction::Quit
        }
        _ => SettingsAction::None,
    }
}
