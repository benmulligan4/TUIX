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

    let value_style = Style::default().fg(Color::White);
    let grey_style = Style::default().fg(Color::DarkGray);

    let items: &[&str] = &[
        "On-Screen Keyboard",
        "Start on Boot",
        "Clear Logs",
    ];

    let mut lines: Vec<Line> = Vec::new();
    for (i, &label) in items.iter().enumerate() {
        let is_active = i == cursor;
        let prefix = if is_active { "  » " } else { "    " };
        let style = if is_active {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            value_style
        };
        let value = match i {
            0 => format!("  {}", if osk_enabled { "Enabled" } else { "Disabled" }),
            1 => {
                if cfg!(target_os = "linux") { "".to_string() }
                else { "  (Raspberry Pi only)".to_string() }
            }
            _ => "".to_string(),
        };
        let row_style = if i == 1 && !cfg!(target_os = "linux") { grey_style } else { style };
        if value.is_empty() {
            lines.push(Line::from(Span::styled(format!("{}{}", prefix, label), row_style)));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("{}{:<24}", prefix, label), row_style),
                Span::styled(value, row_style),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

pub fn item_count() -> usize { 3 }

pub fn handle_enter(cursor: usize) -> bool {
    match cursor {
        0 => {
            let mut settings = persistence::load();
            let current = persistence::get_bool(&settings, "utilities.onscreen_keyboard_enabled", true);
            persistence::set(&mut settings, "utilities.onscreen_keyboard_enabled", serde_json::Value::Bool(!current));
            persistence::save(&settings);
            crate::utilities::logging::settings(&format!("On-Screen Keyboard {}", if !current { "enabled" } else { "disabled" }));
            false
        }
        1 => {
            // Start on Boot — Pi only
            if !cfg!(target_os = "linux") { return false; }
            let exe = std::env::current_exe().map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "/home/pi/TUIX-Rust/target/release/tuix_rust".to_string());
            let wd = std::env::current_dir().map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "/home/pi/TUIX-Rust".to_string());
            let service = format!(
                "[Unit]\nDescription=TUIX Terminal Interface\nAfter=network.target\n\n\
                 [Service]\nType=simple\nWorkingDirectory={wd}\nExecStart={exe}\nRestart=on-failure\n\n\
                 [Install]\nWantedBy=multi-user.target\n"
            );
            let write_ok = std::fs::write("/etc/systemd/system/tuix.service", service).is_ok();
            if write_ok {
                let _ = std::process::Command::new("sudo").args(["systemctl", "daemon-reload"]).status();
                let _ = std::process::Command::new("sudo").args(["systemctl", "enable", "tuix"]).status();
                crate::utilities::logging::settings("Start on boot enabled");
            }
            false
        }
        2 => {
            // Clear logs — truncate tuix.log
            let _ = std::fs::write("tuix.log", "");
            crate::utilities::logging::settings("Logs cleared");
            true // Signal popup
        }
        _ => false,
    }
}
