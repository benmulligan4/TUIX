/// Hotkey settings — assign apps/dashboards to number keys 1-9.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::settings::persistence;

pub fn render(frame: &mut Frame, area: Rect, cursor: usize, _scroll: usize, editing: bool) {
    let settings = persistence::load();
    let value_style = Style::default().fg(Color::White);
    let mut lines: Vec<Line> = Vec::new();

    for key_num in 1..=9usize {
        let path = format!("hotkeys.key_{}", key_num);
        let current = persistence::get(&settings, &path)
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "None".to_string());

        let item_idx = key_num - 1; // cursor is 0-based
        let is_active = cursor == item_idx;
        let prefix = if is_active { "  » " } else { "    " };
        let style = if is_active {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            value_style
        };
        if is_active && editing {
            lines.push(Line::from(vec![
                Span::styled(format!("{}{:<24}", prefix, format!("Hotkey {}", key_num)), style),
                Span::styled("  ◄ ", Style::default().fg(Color::Yellow)),
                Span::styled(current.clone(), Style::default().fg(Color::White)),
                Span::styled(" ►", Style::default().fg(Color::Yellow)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!("{}{:<24}", prefix, format!("Hotkey {}", key_num)), style),
                Span::styled(format!("  {}", current), style),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
}

pub fn is_edit_mode_item(_cursor: usize) -> bool {
    true // all hotkey slots have multiple options
}

pub fn item_count() -> usize { 9 }

pub fn handle_cycle(cursor: usize, forward: bool) {
    let key_num = cursor + 1; // cursor 0 = key_1
    if key_num > 9 { return; }
    let mut settings = persistence::load();
    let path = format!("hotkeys.key_{}", key_num);
    let current = persistence::get(&settings, &path)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "None".to_string());

    let dash_names = persistence::load_dashboard_names();
    let app_names = persistence::load_app_names();
    let mut all_options: Vec<String> = vec!["None".to_string()];
    for d in &dash_names {
        all_options.push(format!("dashboard:{}", d));
    }
    for a in &app_names {
        all_options.push(format!("app:{}", a));
    }

    let idx = all_options.iter().position(|o| o == &current).unwrap_or(0);
    let len = all_options.len();
    let next_idx = if forward { (idx + 1) % len } else { (idx + len - 1) % len };
    let next = &all_options[next_idx];
    let value = if next == "None" {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(next.clone())
    };
    persistence::set(&mut settings, &path, value);
    persistence::save(&settings);
    crate::utilities::logging::settings(&format!("Hotkey {} set to {}", key_num, next));
}

pub fn handle_enter(_cursor: usize) {
    // Hotkeys use edit mode (handle_cycle); nothing to do on Enter
}
