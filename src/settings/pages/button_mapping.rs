/// Button Mapping settings.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::settings::persistence;
use crate::settings::state::SettingsState;

pub fn render(frame: &mut Frame, area: Rect, cursor: usize, _scroll: usize, awaiting_key: bool) {
    let settings = persistence::load();
    let toggle_key = persistence::get_str(&settings, "button_mapping.focus_toggle_key", "Tab");

    let value_style = Style::default().fg(Color::White);
    let mut lines: Vec<Line> = Vec::new();

    let label = "Nav/Main Toggle Key";
    let display_value = if awaiting_key {
        "Press a key...".to_string()
    } else {
        toggle_key
    };

    let prefix = if cursor == 0 { "  » " } else { "    " };
    let style = if cursor == 0 {
        if awaiting_key {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        }
    } else {
        value_style
    };
    lines.push(Line::from(vec![
        Span::styled(format!("{}{:<24}", prefix, label), style),
        Span::styled(format!("  {}", display_value), style),
    ]));

    frame.render_widget(Paragraph::new(lines), area);
}

pub fn item_count() -> usize { 1 }

/// Called when Enter is pressed — starts key capture mode.
pub fn handle_enter(ss: &mut SettingsState) {
    ss.awaiting_key = true;
}

/// Called when a key is captured during awaiting_key mode.
pub fn capture_key(ss: &mut SettingsState, key_name: &str) {
    let mut settings = persistence::load();
    persistence::set(
        &mut settings,
        "button_mapping.focus_toggle_key",
        serde_json::Value::String(key_name.to_string()),
    );
    persistence::save(&settings);
    ss.awaiting_key = false;
}
