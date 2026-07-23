/// Hotkey settings — assign apps/dashboards to number keys 1-9.

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
    let numpad_nav = persistence::get_bool(&settings, "hotkeys.numpad_navigation", false);

    let value_style = Style::default().fg(Color::White);
    let mut lines: Vec<Line> = Vec::new();

    // Numpad navigation toggle (item 0)
    {
        let prefix = if cursor == 0 { "  » " } else { "    " };
        let style = if cursor == 0 {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            value_style
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{}{:<24}", prefix, "Numpad Navigation"), style),
            Span::styled(
                format!("  {}", if numpad_nav { "Enabled" } else { "Disabled" }),
                style,
            ),
        ]));
    }

    lines.push(Line::from(""));

    // Hotkeys 1-9 (items 1..=9)
    let dash_names = persistence::load_dashboard_names();
    let app_names = persistence::load_app_names();
    let mut all_options: Vec<String> = vec!["None".to_string()];
    for d in &dash_names {
        all_options.push(format!("Dashboard: {}", d));
    }
    for a in &app_names {
        all_options.push(format!("App: {}", a));
    }

    for key_num in 1..=9usize {
        let path = format!("hotkeys.key_{}", key_num);
        let current = persistence::get(&settings, &path)
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "None".to_string());

        let item_idx = key_num; // 1-based, but cursor is 0-based starting at numpad toggle
        let prefix = if cursor == item_idx { "  » " } else { "    " };
        let style = if cursor == item_idx {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            value_style
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{}{:<24}", prefix, format!("Hotkey {}", key_num)), style),
            Span::styled(format!("  {}", current), style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

pub fn item_count() -> usize { 10 } // numpad toggle + 9 hotkeys

pub fn handle_enter(cursor: usize) {
    let mut settings = persistence::load();
    if cursor == 0 {
        // Toggle numpad navigation
        let current = persistence::get_bool(&settings, "hotkeys.numpad_navigation", false);
        persistence::set(&mut settings, "hotkeys.numpad_navigation", serde_json::Value::Bool(!current));
        persistence::save(&settings);
        crate::utilities::logging::settings(&format!("Numpad navigation {}", if !current { "enabled" } else { "disabled" }));
        return;
    }

    // Cycle hotkey assignment for key 1-9
    let key_num = cursor; // cursor 1 = key_1, etc.
    if key_num > 9 { return; }

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
    let next = &all_options[(idx + 1) % all_options.len()];
    let value = if next == "None" {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(next.clone())
    };
    persistence::set(&mut settings, &path, value);
    persistence::save(&settings);
    crate::utilities::logging::settings(&format!("Hotkey {} set to {}", key_num, next));
}
