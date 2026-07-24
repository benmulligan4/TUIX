/// Utility settings — on-screen keyboard toggle, start on boot, clear logs.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::settings::persistence;

// Flag file path (relative to $HOME) that ~/.bashrc checks at login.
const AUTOSTART_FLAG: &str = ".config/tuix/autostart";
// Markers wrapping the block we inject into ~/.bashrc.
const MARKER_BEGIN: &str = "# TUIX_AUTOSTART_BEGIN";
const MARKER_END: &str = "# TUIX_AUTOSTART_END";

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/home/pi".to_string())
}

pub fn autostart_is_enabled() -> bool {
    let flag = format!("{}/{}", home(), AUTOSTART_FLAG);
    std::path::Path::new(&flag).exists()
}

fn toggle_autostart() -> String {
    if !cfg!(target_os = "linux") {
        return "Start on Boot is only available on Raspberry Pi.".to_string();
    }

    if autostart_is_enabled() {
        // --- Disable ---
        let flag = format!("{}/{}", home(), AUTOSTART_FLAG);
        let _ = std::fs::remove_file(&flag);

        let bashrc = format!("{}/.bashrc", home());
        if let Ok(content) = std::fs::read_to_string(&bashrc) {
            if let (Some(start), Some(end_tag)) = (
                content.find(MARKER_BEGIN),
                content.find(MARKER_END),
            ) {
                let end = end_tag + MARKER_END.len();
                let new_content = format!(
                    "{}{}",
                    content[..start].trim_end_matches('\n'),
                    content[end..].trim_start_matches('\n')
                );
                let _ = std::fs::write(&bashrc, new_content);
            }
        }
        crate::utilities::logging::settings("Start on boot disabled");
        "Start on boot disabled.".to_string()
    } else {
        // --- Enable ---
        let config_dir = format!("{}/.config/tuix", home());
        let _ = std::fs::create_dir_all(&config_dir);
        let flag = format!("{}/{}", home(), AUTOSTART_FLAG);
        let _ = std::fs::write(&flag, "");

        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| format!("{}/TUIX-Rust/target/release/tuix_rust", home()));
        let wd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| format!("{}/TUIX-Rust", home()));

        let block = format!(
            "\n{}\n# TUIX will start automatically on login when this file exists:\nif [ -f \"$HOME/{}\" ]; then\n  cd \"{}\"\n  exec \"{}\"\nfi\n{}\n",
            MARKER_BEGIN, AUTOSTART_FLAG, wd, exe, MARKER_END
        );

        let bashrc = format!("{}/.bashrc", home());
        let current = std::fs::read_to_string(&bashrc).unwrap_or_default();

        if current.contains(MARKER_BEGIN) {
            crate::utilities::logging::settings("Start on boot enabled (already configured)");
            return "Start on boot enabled.".to_string();
        }

        match std::fs::write(&bashrc, format!("{}{}", current, block)) {
            Ok(_) => {
                crate::utilities::logging::settings("Start on boot enabled");
                "Start on boot enabled.".to_string()
            }
            Err(e) => {
                let msg = format!("Failed to update ~/.bashrc: {}", e);
                crate::utilities::logging::error(&msg);
                msg
            }
        }
    }
}

pub fn render(frame: &mut Frame, area: Rect, cursor: usize, _scroll: usize) {
    let settings = persistence::load();
    let osk_enabled = persistence::get_bool(&settings, "utilities.onscreen_keyboard_enabled", true);
    let boot_enabled = autostart_is_enabled();

    let value_style = Style::default().fg(Color::White);
    let grey_style = Style::default().fg(Color::DarkGray);

    let items: &[(&str, String)] = &[
        ("On-Screen Keyboard", if osk_enabled { "Enabled".into() } else { "Disabled".into() }),
        (
            "Start on Boot",
            if !cfg!(target_os = "linux") {
                "(Raspberry Pi only)".into()
            } else if boot_enabled {
                "Enabled".into()
            } else {
                "Disabled".into()
            },
        ),
        ("Clear Logs", "".into()),
    ];

    let mut lines: Vec<Line> = Vec::new();
    for (i, (label, value)) in items.iter().enumerate() {
        let is_active = i == cursor;
        let unavailable = i == 1 && !cfg!(target_os = "linux");
        let prefix = if is_active && !unavailable { "  » " } else { "    " };
        let style = if is_active && !unavailable {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else if unavailable {
            grey_style
        } else {
            value_style
        };
        if value.is_empty() {
            lines.push(Line::from(Span::styled(format!("{}{}", prefix, label), style)));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("{}{:<24}", prefix, label), style),
                Span::styled(format!("  {}", value), style),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

pub fn item_count() -> usize { 3 }

/// Returns Some(popup_message) when a popup should be shown, None otherwise.
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
            // Start on Boot toggle — Pi only
            let _msg = toggle_autostart();
            false // logging is done inside toggle_autostart
        }
        2 => {
            let _ = std::fs::write("tuix.log", "");
            crate::utilities::logging::settings("Logs cleared");
            true // Show popup
        }
        _ => false,
    }
}
