/// Awesome Ratatui App Browser — right-pane rendering when the browser is active.

use std::collections::HashMap;

use ratatui::{
    layout::{Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use serde_json::Value;

use super::awesome_ratatui_manager::{BrowserFocus, BrowserState, RowKind};
use super::state::InstallStatus;

pub fn render_browser(
    frame: &mut Frame,
    area: Rect,
    browser: &BrowserState,
    registered: &HashMap<String, Value>,
    statuses: &HashMap<String, InstallStatus>,
) {
    let right_block = Block::default()
        .borders(Borders::LEFT)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(right_block, area);

    let inner = area.inner(Margin { horizontal: 1, vertical: 0 });

    if browser.loading {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "\n  Fetching awesome-ratatui list...",
                Style::default().fg(Color::Yellow),
            ))),
            inner,
        );
        return;
    }

    if let Some(err) = &browser.error {
        let lines = vec![
            Line::from(Span::styled(
                " Awesome Ratatui App Browser",
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("  Error: {}", err),
                Style::default().fg(Color::Red),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Press Esc to go back",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        frame.render_widget(Paragraph::new(lines), inner);
        return;
    }

    if browser.focus == BrowserFocus::Actions {
        render_app_detail(frame, inner, browser, registered, statuses);
        return;
    }

    render_app_list(frame, inner, browser, registered, statuses);
}

fn render_app_list(
    frame: &mut Frame,
    area: Rect,
    browser: &BrowserState,
    registered: &HashMap<String, Value>,
    statuses: &HashMap<String, InstallStatus>,
) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        " Awesome Ratatui App Browser",
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    )));

    if let Some(ts) = browser.fetched_at {
        let age_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(ts);
        let age_str = if age_secs < 60 {
            format!("{}s ago", age_secs)
        } else if age_secs < 3600 {
            format!("{}m ago", age_secs / 60)
        } else {
            format!("{}h ago", age_secs / 3600)
        };
        lines.push(Line::from(Span::styled(
            format!("  Last updated: {}", age_str),
            Style::default().fg(Color::DarkGray),
        )));
    }
    lines.push(Line::from(""));

    // Search bar
    let search_style = if browser.focus == BrowserFocus::SearchBar {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::default().fg(Color::Cyan)
    };
    let search_text = if browser.focus == BrowserFocus::SearchBar {
        if browser.search_query.is_empty() {
            " [/] Type to search...".to_string()
        } else {
            format!(" [/] {}|", browser.search_query)
        }
    } else if browser.search_query.is_empty() {
        " [/] Search".to_string()
    } else {
        format!(" [/] {}", browser.search_query)
    };
    lines.push(Line::from(Span::styled(search_text, search_style)));

    // Show descriptions toggle
    let toggle_style = if browser.focus == BrowserFocus::DescriptionToggle {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let desc_check = if browser.show_descriptions { "[*]" } else { "[ ]" };
    lines.push(Line::from(Span::styled(
        format!(" {}  Show Descriptions", desc_check),
        toggle_style,
    )));

    lines.push(Line::from(Span::styled(
        " ─────────────────────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));

    let header_lines = lines.len();

    let available = area.height as usize;
    let visible_height = available.saturating_sub(header_lines);
    let total = browser.visible_rows.len();

    let scroll = browser.scroll.min(total.saturating_sub(visible_height));
    let end = (scroll + visible_height).min(total);

    for i in scroll..end {
        let row = &browser.visible_rows[i];
        let is_cursor = i == browser.cursor && browser.focus == BrowserFocus::List;

        match row {
            RowKind::Spacer => {
                lines.push(Line::from(""));
            }
            RowKind::CategoryHeader(cat) => {
                let collapsed = browser.collapsed.contains(cat);
                let arrow = if collapsed { "▶" } else { "▼" };
                let cat_style = if is_cursor {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                };
                let count = browser.apps.iter()
                    .filter(|a| a.category == *cat)
                    .filter(|a| {
                        browser.search_query.is_empty()
                            || a.name.to_lowercase().contains(&browser.search_query.to_lowercase())
                            || a.description.to_lowercase().contains(&browser.search_query.to_lowercase())
                    })
                    .count();
                lines.push(Line::from(Span::styled(
                    format!(" {} {} ({})", arrow, cat, count),
                    cat_style,
                )));
            }
            RowKind::App(idx) => {
                let app = &browser.apps[*idx];
                let key = super::awesome_ratatui_manager::repo_to_key(&app.repo_url);
                let is_reg = registered.contains_key(&key);
                let is_inst = matches!(
                    statuses.get(&key),
                    Some(s) if !matches!(s, InstallStatus::NotInstalled)
                );

                let prefix = if is_cursor { " » " } else { "   " };
                let suffix = if is_inst {
                    " (installed)"
                } else if is_reg {
                    " (registered)"
                } else {
                    ""
                };

                let color = if is_inst {
                    Color::Green
                } else if is_reg {
                    Color::Magenta
                } else {
                    Color::White
                };

                let style = if is_cursor {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(color)
                };

                if browser.show_descriptions && !app.description.is_empty() {
                    let desc_style = if is_cursor {
                        Style::default().fg(Color::Black).bg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    let max_w = area.width.saturating_sub(4) as usize;
                    let name_part = format!("{}{}{}", prefix, app.name, suffix);
                    let sep = " - ";
                    let remaining = max_w.saturating_sub(name_part.len() + sep.len());
                    let desc_text = if app.description.len() > remaining && remaining > 3 {
                        format!("{}...", &app.description[..remaining.saturating_sub(3)])
                    } else {
                        app.description.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::styled(name_part, style),
                        Span::styled(format!("{}{}", sep, desc_text), desc_style),
                    ]));
                } else {
                    let name_line = format!("{}{}{}", prefix, app.name, suffix);
                    lines.push(Line::from(Span::styled(name_line, style)));
                }
            }
        }
    }

    if browser.visible_rows.is_empty() {
        lines.push(Line::from(Span::styled(
            "   No apps match search",
            Style::default().fg(Color::DarkGray),
        )));
    }

    if total > visible_height {
        let indicator = format!(" {}/{}", browser.cursor + 1, total);
        lines.push(Line::from(Span::styled(
            indicator,
            Style::default().fg(Color::DarkGray),
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_app_detail(
    frame: &mut Frame,
    area: Rect,
    browser: &BrowserState,
    registered: &HashMap<String, Value>,
    statuses: &HashMap<String, InstallStatus>,
) {
    let app = match browser.selected_app() {
        Some(a) => a,
        None => return,
    };

    let key = super::awesome_ratatui_manager::repo_to_key(&app.repo_url);
    let is_reg = registered.contains_key(&key);
    let is_inst = matches!(
        statuses.get(&key),
        Some(s) if !matches!(s, InstallStatus::NotInstalled)
    );

    let label_style = Style::default().fg(Color::DarkGray);
    let value_style = Style::default().fg(Color::White);
    let title_style = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
    let accent_style = Style::default().fg(Color::Cyan);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(&app.name, title_style)));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("  Category:    ", label_style),
        Span::styled(&app.category, value_style),
    ]));

    let author_name = app.repo_url
        .trim_end_matches('/')
        .rsplitn(3, '/')
        .nth(1)
        .unwrap_or("unknown");
    lines.push(Line::from(vec![
        Span::styled("  Author:      ", label_style),
        Span::styled(author_name, value_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Repository:  ", label_style),
        Span::styled(&app.repo_url, accent_style),
    ]));
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("  Description:", label_style)));
    let max_width = area.width.saturating_sub(4) as usize;
    for desc_line in super::page::wrap_text(&app.description, max_width) {
        lines.push(Line::from(Span::styled(
            format!("  {}", desc_line),
            value_style,
        )));
    }
    lines.push(Line::from(""));

    let (status_text, status_color) = if is_inst {
        ("Installed".to_string(), Color::Green)
    } else if is_reg {
        ("Registered (not installed)".to_string(), Color::Magenta)
    } else {
        ("Not in app store".to_string(), Color::DarkGray)
    };
    lines.push(Line::from(vec![
        Span::styled("  Status:      ", label_style),
        Span::styled(&status_text, Style::default().fg(status_color)),
    ]));
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("  Actions:", label_style)));
    lines.push(Line::from(""));

    let actions = build_browser_actions(is_reg, is_inst);
    for (i, action_label) in actions.iter().enumerate() {
        let color = match action_label.as_str() {
            "Open Repository" | "Open in App Store" => accent_style,
            "Add to App Store" | "Add to App Store & Install" => Style::default().fg(Color::Green),
            "Remove from App Store" => Style::default().fg(Color::Red),
            _ => accent_style,
        };
        let style = action_button_style(browser.action_cursor == i, color);
        lines.push(Line::from(Span::styled(
            format!("  [ {} ]", action_label),
            style,
        )));
    }

    let visible_height = area.height as usize;
    let total_lines = lines.len();
    let end = visible_height.min(total_lines);
    let visible: Vec<Line> = if total_lines > visible_height {
        lines[..end].to_vec()
    } else {
        lines
    };

    frame.render_widget(Paragraph::new(visible), area);
}

fn action_button_style(selected: bool, default: Style) -> Style {
    if selected {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        default
    }
}

/// Build the list of action labels for an app in the browser detail view.
pub fn build_browser_actions(is_registered: bool, is_installed: bool) -> Vec<String> {
    let mut actions = vec!["Open Repository".to_string()];

    if !is_registered {
        actions.push("Add to App Store".to_string());
        actions.push("Add to App Store & Install".to_string());
    } else if is_installed {
        actions.push("Open in App Store".to_string());
    } else {
        actions.push("Remove from App Store".to_string());
        actions.push("Open in App Store".to_string());
    }

    actions
}
