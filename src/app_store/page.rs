/// App Store page — rendered in the TUIX main container.
///
/// Left pane: search bar, sort/filter controls, scrollable app list.
/// Right pane: app preview with metadata and install/uninstall actions.
/// Bottom: terminal panel during operations.

use std::collections::HashMap;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use serde_json::Value;

use super::state::{AppStoreFocus, AppStoreState, ConfirmAction, InstallStatus};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    border_style: Style,
    state: &AppStoreState,
    registered: &HashMap<String, Value>,
) {
    let border_name = {
        let s = crate::settings::persistence::load();
        crate::settings::persistence::get_str(&s, "appearance.border_style", "Rounded")
    };
    let btype = crate::settings::pages::appearance::border_type_from_name(&border_name);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(btype)
        .title(" App Store ")
        .title_alignment(Alignment::Right)
        .style(border_style);
    frame.render_widget(block, area);

    let inner = area.inner(Margin { horizontal: 1, vertical: 1 });

    // Split main area into left and right panes
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(32), Constraint::Fill(1)])
        .split(inner);

    let left_area = panes[0];
    let full_right_area = panes[1];

    render_left_pane(frame, left_area, state, registered);

    // If browser is active, render it in the right pane
    if state.browser.active {
        super::awesome_ratatui_page::render_browser(
            frame, full_right_area, &state.browser, registered, &state.install_statuses,
        );
    } else if state.terminal_visible {
        let right_sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(full_right_area);
        render_right_pane(frame, right_sections[0], state, registered);
        render_terminal_panel(frame, right_sections[1], state);
    } else {
        render_right_pane(frame, full_right_area, state, registered);
    }

    // Render overlay dialogs
    if state.confirm_dialog.is_some() {
        render_confirm_dialog(frame, area, state);
    }
    if state.install_location_dialog {
        render_install_location_dialog(frame, area, state);
    }
}

fn render_left_pane(
    frame: &mut Frame,
    area: Rect,
    state: &AppStoreState,
    registered: &HashMap<String, Value>,
) {
    let in_left = matches!(
        state.focus,
        AppStoreFocus::LeftPane | AppStoreFocus::SearchBar | AppStoreFocus::FilterPanel | AppStoreFocus::BrowserButton
    );

    let mut lines: Vec<Line> = Vec::new();

    // Search bar
    let search_style = if state.focus == AppStoreFocus::SearchBar {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else if in_left {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let search_text = if state.focus == AppStoreFocus::SearchBar {
        if state.search_query.is_empty() {
            " [/] Type to search...".to_string()
        } else {
            format!(" [/] {}|", state.search_query)
        }
    } else if state.search_query.is_empty() {
        " [/] Search".to_string()
    } else {
        format!(" [/] {}", state.search_query)
    };
    lines.push(Line::from(Span::styled(search_text, search_style)));

    // Awesome Ratatui App Browser button
    let browser_active = state.browser.active;
    let browser_btn_focused = state.focus == AppStoreFocus::BrowserButton;
    let browser_style = if browser_btn_focused {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else if browser_active {
        Style::default().fg(Color::Black).bg(Color::Green)
    } else if in_left {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(Span::styled(
        if browser_active { " ✚ Browse Awesome Ratatui  [open]" } else { " ✚ Browse Awesome Ratatui" },
        browser_style,
    )));

    // Filter panel toggle
    let panel_focused = state.focus == AppStoreFocus::FilterPanel;
    let arrow = if state.filter_panel_open { "▼" } else { "▶" };
    let panel_style = if panel_focused && state.filter_panel_cursor == 0 {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else if in_left {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(Span::styled(
        format!(" {} Filters & Sort", arrow),
        panel_style,
    )));

    if state.filter_panel_open {
        let categories = AppStoreState::all_categories(registered);

        let item_style = |idx: usize| -> Style {
            if panel_focused && state.filter_panel_cursor == idx {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            }
        };
        let label_style = Style::default().fg(Color::DarkGray);

        // Sort mode (cursor 1)
        let sort_arrow = if state.sort_dropdown_open { "▲" } else { "▼" };
        lines.push(Line::from(Span::styled(
            format!("   Sort: {} {}", state.sort_mode.label(), sort_arrow),
            item_style(1),
        )));

        // Sort dropdown (inline, shown when open)
        if state.sort_dropdown_open {
            for (si, mode) in crate::app_store::state::SortMode::ALL.iter().enumerate() {
                let prefix = if si == state.sort_dropdown_cursor { "    » " } else { "      " };
                let s = if si == state.sort_dropdown_cursor {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                lines.push(Line::from(Span::styled(
                    format!("{}{}", prefix, mode.label()),
                    s,
                )));
            }
        }

        // Show Installed (cursor 2)
        let i_check = if state.filter_show_installed { "[*]" } else { "[ ]" };
        lines.push(Line::from(Span::styled(
            format!("   {}  Show Installed", i_check),
            item_style(2),
        )));

        // Show Uninstalled (cursor 3)
        let u_check = if state.filter_show_uninstalled { "[*]" } else { "[ ]" };
        lines.push(Line::from(Span::styled(
            format!("   {}  Show Uninstalled", u_check),
            item_style(3),
        )));

        // Category filters (cursor 4..4+N-1)
        if !categories.is_empty() {
            lines.push(Line::from(Span::styled("   Categories:", label_style)));
        }
        for (ci, cat) in categories.iter().enumerate() {
            let is_shown = !state.filter_categories.contains(cat);
            let check = if is_shown { "[*]" } else { "[ ]" };
            lines.push(Line::from(Span::styled(
                format!("    {}  {}", check, cat),
                item_style(4 + ci),
            )));
        }
    }

    // Refresh button (always visible, outside the panel)
    let refresh_idx = state.refresh_cursor_idx(registered);
    let refresh_style = if panel_focused && state.filter_panel_cursor == refresh_idx {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(Span::styled(
        " ↻ Refresh installed apps",
        refresh_style,
    )));

    lines.push(Line::from(Span::styled(
        " ─────────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));

    // App list
    let available_rows = area.height as usize;

    for (i, key) in state.computed_app_list.iter().enumerate() {
        if lines.len() >= available_rows {
            break;
        }

        let meta = match registered.get(key.as_str()) {
            Some(m) => m,
            None => continue,
        };
        let label = meta.get("label").and_then(|v| v.as_str()).unwrap_or(key);
        let is_selected = i == state.left_cursor;
        let in_app_list = state.focus == AppStoreFocus::LeftPane;

        let installed_indicator = match state.install_statuses.get(key.as_str()) {
            Some(InstallStatus::NotInstalled) | None => " ",
            _ => "✓",
        };

        let prefix = if is_selected && in_app_list { " » " } else { "   " };
        let text = format!("{}{} {}", prefix, installed_indicator, label);

        let is_installed = matches!(
            state.install_statuses.get(key.as_str()),
            Some(s) if !matches!(s, InstallStatus::NotInstalled)
        );
        let is_failed = state.failed_installs.contains(key);

        let style = if is_selected && in_app_list {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else if is_failed {
            Style::default().fg(Color::Red)
        } else if is_installed {
            Style::default().fg(Color::Green)
        } else if is_selected {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };

        lines.push(Line::from(Span::styled(text, style)));
    }

    if state.computed_app_list.is_empty() && lines.len() < available_rows {
        lines.push(Line::from(Span::styled(
            "   No apps match filters",
            Style::default().fg(Color::DarkGray),
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_right_pane(
    frame: &mut Frame,
    area: Rect,
    state: &AppStoreState,
    registered: &HashMap<String, Value>,
) {
    let right_block = Block::default()
        .borders(Borders::LEFT)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(right_block, area);

    let right_inner = area.inner(Margin { horizontal: 2, vertical: 0 });

    let selected_key = match state.selected_app_key() {
        Some(k) => k.clone(),
        None => {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "\n  Please select an app to preview details",
                    Style::default().fg(Color::DarkGray),
                ))),
                right_inner,
            );
            return;
        }
    };

    let meta = match registered.get(&selected_key) {
        Some(m) => m,
        None => return,
    };

    let label = meta.get("label").and_then(|v| v.as_str()).unwrap_or(&selected_key);
    let category = meta.get("category").and_then(|v| v.as_str()).unwrap_or("Other");
    let author = meta.get("author").and_then(|v| v.as_str()).unwrap_or("Unknown");
    let version = meta.get("version").and_then(|v| v.as_str()).unwrap_or("—");
    let license = meta.get("license").and_then(|v| v.as_str()).unwrap_or("—");
    let description = meta.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let repository = meta.get("repository").and_then(|v| v.as_str()).unwrap_or("");

    let install_status = state
        .install_statuses
        .get(&selected_key)
        .cloned()
        .unwrap_or(InstallStatus::NotInstalled);

    let label_style = Style::default().fg(Color::DarkGray);
    let value_style = Style::default().fg(Color::White);
    let title_style = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
    let accent_style = Style::default().fg(Color::Cyan);

    let is_approved = meta.get("approved").and_then(|v| v.as_bool()).unwrap_or(true);

    let mut lines: Vec<Line> = Vec::new();

    // Title
    lines.push(Line::from(Span::styled(label, title_style)));

    // Approved badge or experimental warning
    if is_approved {
        lines.push(Line::from(Span::styled(
            "  ✓ TUIX Approved",
            Style::default().fg(Color::Green),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  ⚠ Experimental — added from Awesome Ratatui",
            Style::default().fg(Color::Yellow),
        )));
    }
    lines.push(Line::from(""));

    // Metadata grid
    lines.push(Line::from(vec![
        Span::styled("  Category:    ", label_style),
        Span::styled(category, value_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Author:      ", label_style),
        Span::styled(author, value_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Version:     ", label_style),
        Span::styled(version, value_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  License:     ", label_style),
        Span::styled(license, value_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Repository:  ", label_style),
        Span::styled(repository, accent_style),
    ]));
    lines.push(Line::from(""));

    // Description
    lines.push(Line::from(Span::styled("  Description:", label_style)));
    // Word-wrap description
    let max_width = right_inner.width.saturating_sub(4) as usize;
    for desc_line in wrap_text(description, max_width) {
        lines.push(Line::from(Span::styled(
            format!("  {}", desc_line),
            value_style,
        )));
    }
    lines.push(Line::from(""));

    // Install status
    let (status_text, status_color) = match &install_status {
        InstallStatus::NotInstalled => ("Not installed".to_string(), Color::DarkGray),
        InstallStatus::Global(path) => (format!("Installed to PATH: {}", path), Color::Green),
        InstallStatus::Local(path) => (format!("Installed to Downloads: {}", path), Color::Green),
        InstallStatus::Both(_, _) => ("Installed to Both".to_string(), Color::Green),
    };

    lines.push(Line::from(vec![
        Span::styled("  Status:      ", label_style),
        Span::styled(&status_text, Style::default().fg(status_color)),
    ]));

    if let InstallStatus::Both(g, l) = &install_status {
        lines.push(Line::from(Span::styled(
            format!("    PATH:      {}", g),
            Style::default().fg(Color::Green),
        )));
        lines.push(Line::from(Span::styled(
            format!("    Downloads: {}", l),
            Style::default().fg(Color::Green),
        )));
    }

    lines.push(Line::from(""));

    // Action buttons
    lines.push(Line::from(Span::styled("  Actions:", label_style)));
    lines.push(Line::from(""));

    let in_actions = state.focus == AppStoreFocus::RightPane && state.in_right_actions;
    let mut action_idx: usize = 0;

    let install_methods: Vec<&str> = meta.get("install_methods")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_else(|| vec!["global"]);
    let supports_local = install_methods.contains(&"local");
    let supports_global = install_methods.contains(&"global");

    // Run button (only for installed apps)
    let is_installed = !matches!(install_status, InstallStatus::NotInstalled);
    if is_installed {
        let run_style = if in_actions && state.right_action_cursor == action_idx {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        };
        lines.push(Line::from(Span::styled("  [ Run ]", run_style)));
        action_idx += 1;
    }

    // Open Repository
    let repo_style = if in_actions && state.right_action_cursor == action_idx {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        accent_style
    };
    lines.push(Line::from(Span::styled("  [ Open Repository ]", repo_style)));
    action_idx += 1;

    // Install / Uninstall buttons based on status
    match &install_status {
        InstallStatus::NotInstalled => {
            let install_style = if in_actions && state.right_action_cursor == action_idx {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::Green)
            };
            lines.push(Line::from(Span::styled("  [ Install ]", install_style)));
        }
        InstallStatus::Global(_) => {
            let uninstall_style = if in_actions && state.right_action_cursor == action_idx {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::Red)
            };
            lines.push(Line::from(Span::styled("  [ Uninstall from PATH ]", uninstall_style)));
            action_idx += 1;

            if supports_local {
                let install_local_style = if in_actions && state.right_action_cursor == action_idx {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Green)
                };
                lines.push(Line::from(Span::styled("  [ Install to Downloads ]", install_local_style)));
                action_idx += 1;
            } else {
                lines.push(Line::from(Span::styled(
                    "    (local install not supported for this app)",
                    Style::default().fg(Color::DarkGray),
                )));
            }

            let open_style = if in_actions && state.right_action_cursor == action_idx {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                accent_style
            };
            lines.push(Line::from(Span::styled("  [ Open Install Location ]", open_style)));
        }
        InstallStatus::Local(_) => {
            let uninstall_style = if in_actions && state.right_action_cursor == action_idx {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::Red)
            };
            lines.push(Line::from(Span::styled("  [ Uninstall from Downloads ]", uninstall_style)));
            action_idx += 1;

            if supports_global {
                let install_path_style = if in_actions && state.right_action_cursor == action_idx {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Green)
                };
                lines.push(Line::from(Span::styled("  [ Install to PATH ]", install_path_style)));
                action_idx += 1;
            } else {
                lines.push(Line::from(Span::styled(
                    "    (PATH install not supported for this app)",
                    Style::default().fg(Color::DarkGray),
                )));
            }

            let open_style = if in_actions && state.right_action_cursor == action_idx {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                accent_style
            };
            lines.push(Line::from(Span::styled("  [ Open Install Location ]", open_style)));
        }
        InstallStatus::Both(_, _) => {
            let uninstall_style = if in_actions && state.right_action_cursor == action_idx {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::Red)
            };
            lines.push(Line::from(Span::styled("  [ Uninstall ]", uninstall_style)));
            action_idx += 1;

            // Run source selector
            let current_source = super::actions::get_run_source(&selected_key);
            let source_label = if current_source == "local" { "Downloads" } else { "PATH" };
            let source_style = if in_actions && state.right_action_cursor == action_idx {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::Yellow)
            };
            let source_text = format!("  [ Default Source: {} — press to switch ]", source_label);
            lines.push(Line::from(Span::styled(source_text, source_style)));
            if current_source == "local" {
                lines.push(Line::from(Span::styled(
                    "    ⚠ Downloads source is experimental",
                    Style::default().fg(Color::Yellow),
                )));
            }
            action_idx += 1;

            let open_path_style = if in_actions && state.right_action_cursor == action_idx {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                accent_style
            };
            lines.push(Line::from(Span::styled("  [ Open PATH Location ]", open_path_style)));
            action_idx += 1;

            let open_local_style = if in_actions && state.right_action_cursor == action_idx {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                accent_style
            };
            lines.push(Line::from(Span::styled("  [ Open Downloads Location ]", open_local_style)));
        }
    }

    // Window mode setting (for installed apps)
    if is_installed {
        let supports_embed = super::actions::supports_embedded(registered.get(&selected_key));
        lines.push(Line::from(""));
        action_idx += 1;

        if !supports_embed {
            lines.push(Line::from(Span::styled(
                "  ⚠ This app only supports fullscreen mode",
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(Span::styled(
                "    (running inside TUIX is not supported yet)",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            let current_mode = super::actions::get_window_mode(&selected_key, registered.get(&selected_key));
            let mode_label = if current_mode == "fullscreen" { "Fullscreen" } else { "TUIX Container" };
            let mode_style = if in_actions && state.right_action_cursor == action_idx {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::Yellow)
            };
            lines.push(Line::from(Span::styled(
                format!("  [ Window Mode: {} — press to switch ]", mode_label),
                mode_style,
            )));
        }
    }

    // Remove from App Store (only for non-approved, not-installed apps)
    if !is_approved && !is_installed {
        action_idx += 1;
        let remove_style = if in_actions && state.right_action_cursor == action_idx {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::Red)
        };
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  [ Remove from App Store ]", remove_style)));
    }

    // Apply scroll offset
    let visible_height = right_inner.height as usize;
    let total_lines = lines.len();
    let scroll = state.right_scroll.min(total_lines.saturating_sub(visible_height));
    let end = (scroll + visible_height).min(total_lines);
    let visible_lines: Vec<Line> = if total_lines > visible_height {
        lines[scroll..end].to_vec()
    } else {
        lines
    };

    frame.render_widget(Paragraph::new(visible_lines), right_inner);
}

fn render_terminal_panel(frame: &mut Frame, area: Rect, state: &AppStoreState) {
    let border_style = if state.terminal_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if state.operation_running {
        " Terminal Output  [UI Locked — operation in progress] "
    } else if state.terminal_focused {
        " Terminal Output  [Shift+Tab to exit] [Q to close] "
    } else if !state.terminal_output.is_empty() {
        " Terminal Output  [Shift+Tab to enter] [Q to close] "
    } else {
        " Terminal Output  [Shift+Tab to enter] "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(border_style);
    frame.render_widget(block, area);

    let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
    let visible_height = inner.height as usize;

    let mut lines: Vec<Line> = Vec::new();

    if state.terminal_output.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (Install or uninstall an app to see output here)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let max_scroll = state.terminal_output.len().saturating_sub(visible_height);
        let scroll = state.terminal_scroll.min(max_scroll);
        let end = (scroll + visible_height).min(state.terminal_output.len());
        for line in &state.terminal_output[scroll..end] {
            lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::White),
            )));
        }

        // Scroll indicator
        if state.terminal_focused && state.terminal_output.len() > visible_height {
            let info = format!(" {}/{} ", scroll + 1, state.terminal_output.len());
            let info_width = info.len() as u16;
            if area.width > info_width + 2 {
                let indicator_rect = Rect {
                    x: area.x + area.width - info_width - 1,
                    y: area.y + area.height - 1,
                    width: info_width,
                    height: 1,
                };
                frame.render_widget(
                    Paragraph::new(info).style(Style::default().fg(Color::DarkGray)),
                    indicator_rect,
                );
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_confirm_dialog(frame: &mut Frame, area: Rect, state: &AppStoreState) {
    let confirm = match &state.confirm_dialog {
        Some(c) => c,
        None => return,
    };

    let is_choose = matches!(confirm, ConfirmAction::UninstallChoose(_));
    let app_name = match confirm {
        ConfirmAction::UninstallGlobal(n)
        | ConfirmAction::UninstallLocal(n)
        | ConfirmAction::UninstallChoose(n) => n.as_str(),
    };

    if is_choose {
        let msg = format!("Uninstall {} from:", app_name);
        let width = (msg.len() + 10).max(40).min(area.width as usize) as u16;
        let height = 8;
        let popup_rect = Rect {
            x: area.x + (area.width.saturating_sub(width)) / 2,
            y: area.y + (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };
        frame.render_widget(Clear, popup_rect);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(" Uninstall ")
                .style(Style::default().fg(Color::Red).bg(Color::Black)),
            popup_rect,
        );
        let inner = popup_rect.inner(Margin { horizontal: 1, vertical: 1 });
        let options = ["PATH only", "Downloads only", "Both", "Cancel"];
        let mut lines = vec![
            Line::from(Span::styled(format!("  {}", msg), Style::default().fg(Color::White))),
            Line::from(""),
        ];
        for (i, opt) in options.iter().enumerate() {
            let style = if i == state.confirm_cursor {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if i == state.confirm_cursor { "  » " } else { "    " };
            lines.push(Line::from(Span::styled(format!("{}{}", prefix, opt), style)));
        }
        frame.render_widget(Paragraph::new(lines), inner);
    } else {
        let msg = format!("Are you sure you want to uninstall {}?", app_name);
        let width = (msg.len() + 6).min(area.width as usize) as u16;
        let height = 5;
        let popup_rect = Rect {
            x: area.x + (area.width.saturating_sub(width)) / 2,
            y: area.y + (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };
        frame.render_widget(Clear, popup_rect);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm Uninstall ")
                .style(Style::default().fg(Color::Red).bg(Color::Black)),
            popup_rect,
        );
        let inner = popup_rect.inner(Margin { horizontal: 1, vertical: 1 });
        let yes_style = if state.confirm_cursor == 0 {
            Style::default().fg(Color::Black).bg(Color::Red)
        } else {
            Style::default().fg(Color::Red)
        };
        let no_style = if state.confirm_cursor == 1 {
            Style::default().fg(Color::Black).bg(Color::Green)
        } else {
            Style::default().fg(Color::Green)
        };
        let lines = vec![
            Line::from(Span::styled(format!("  {}", msg), Style::default().fg(Color::White))),
            Line::from(""),
            Line::from(vec![
                Span::styled("     ", Style::default()),
                Span::styled(" Yes ", yes_style),
                Span::styled("    ", Style::default()),
                Span::styled(" No ", no_style),
            ]),
        ];
        frame.render_widget(Paragraph::new(lines), inner);
    }
}

fn render_install_location_dialog(frame: &mut Frame, area: Rect, state: &AppStoreState) {
    let width = 50u16.min(area.width);
    let height = (state.available_install_methods.len() as u16 + 4).min(area.height);

    let popup_rect = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    frame.render_widget(Clear, popup_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Choose Install Location ")
        .style(Style::default().fg(Color::Cyan).bg(Color::Black));
    frame.render_widget(block, popup_rect);

    let inner = popup_rect.inner(Margin { horizontal: 1, vertical: 1 });

    let mut lines = vec![
        Line::from(Span::styled(
            "  Where would you like to install?",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
    ];

    for (i, method) in state.available_install_methods.iter().enumerate() {
        let style = if i == state.install_location_cursor {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };
        let prefix = if i == state.install_location_cursor { "  » " } else { "    " };
        lines.push(Line::from(Span::styled(
            format!("{}{}", prefix, method.label()),
            style,
        )));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

/// Simple word-wrap helper.
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    let mut result = Vec::new();
    let mut current_line = String::new();
    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 > max_width && !current_line.is_empty() {
            result.push(current_line);
            current_line = String::new();
        }
        if !current_line.is_empty() {
            current_line.push(' ');
        }
        current_line.push_str(word);
    }
    if !current_line.is_empty() {
        result.push(current_line);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

/// Get the number of action buttons for the current app's install status.
pub fn action_count(status: &InstallStatus, supports_embed: bool, supports_local: bool, supports_global: bool, is_approved: bool) -> usize {
    let run_btn = if matches!(status, InstallStatus::NotInstalled) { 0 } else { 1 };
    let mode_btn = if !matches!(status, InstallStatus::NotInstalled) && supports_embed { 1 } else { 0 };
    let remove_btn = if !is_approved && matches!(status, InstallStatus::NotInstalled) { 1 } else { 0 };
    let base = match status {
        InstallStatus::NotInstalled => 2,  // Open Repo, Install
        InstallStatus::Global(_) => {
            if supports_local { 4 } else { 3 }
        }
        InstallStatus::Local(_) => {
            if supports_global { 4 } else { 3 }
        }
        InstallStatus::Both(_, _) => 5,
    };
    run_btn + base + mode_btn + remove_btn
}
