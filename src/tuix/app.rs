/// TUIX — main shell: event loop, custom navbar, and main container rendering.

use std::io;
use std::process::Command;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use serde_json::Value;

use super::input_handler::{action_to_key_name, map_key};
use super::models::{Action, AppStatus, FocusTarget, TuixState};
use super::process_manager;
use super::registry;

use crate::applications::character_set::{AppAction, CharacterSetApp};
use crate::applications::file_explorer::{FileExplorerAction, FileExplorerApp};
use crate::applications::text_editor::{TextEditorAction, TextEditorApp};
use crate::dashboards::{dashboard_1, dashboard_2};
use crate::dashboards::installed_runner::InstalledDashboard;
use crate::settings;
use crate::touchscreen;
use crate::utilities::logging;

// ---------------------------------------------------------------------------
// Internal app wrapper — supports multiple built-in app types
// ---------------------------------------------------------------------------

enum InternalApp {
    CharacterSet(CharacterSetApp),
    FileExplorer(FileExplorerApp),
    TextEditor(TextEditorApp),
}

impl InternalApp {
    fn render(&mut self, frame: &mut Frame, area: Rect, border_style: Style) {
        match self {
            InternalApp::CharacterSet(app) => app.render(frame, area, border_style),
            InternalApp::FileExplorer(app) => app.render(frame, area, border_style),
            InternalApp::TextEditor(app) => app.render(frame, area, border_style),
        }
    }

    fn handle_key(&mut self, code: &str) -> bool {
        // Returns true if the app wants to close
        match self {
            InternalApp::CharacterSet(app) => {
                matches!(app.handle_key(code), Some(AppAction::Back))
            }
            InternalApp::FileExplorer(app) => {
                matches!(app.handle_key(code), Some(FileExplorerAction::Back))
            }
            InternalApp::TextEditor(app) => {
                matches!(app.handle_key(code), Some(TextEditorAction::Back))
            }
        }
    }

    fn stop(&mut self) {
        match self {
            InternalApp::CharacterSet(app) => app.stop(),
            InternalApp::FileExplorer(app) => app.stop(),
            InternalApp::TextEditor(app) => app.stop(),
        }
    }

    /// Returns true if this app uses the OSK and has it active,
    /// meaning WASD should NOT be mapped to navigation.
    fn wants_raw_wasd(&self) -> bool {
        match self {
            InternalApp::TextEditor(app) => app.keyboard_active(),
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Navbar definition
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum NavContent {
    /// Direct action string (e.g., "page:settings")
    Direct(String),
    /// Dropdown children: vec of (label, action_string)
    Children(Vec<(String, String)>),
}

#[derive(Debug, Clone)]
pub struct NavItem {
    pub label: String,
    pub content: NavContent,
}

fn build_nav_items(
    dashboards: &std::collections::HashMap<String, Value>,
    apps_registry: &std::collections::HashMap<String, Value>,
) -> Vec<NavItem> {
    let dash_children: Vec<(String, String)> = dashboards
        .iter()
        .map(|(name, meta)| {
            let label = meta
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or(name)
                .to_string();
            (label, format!("dashboard:{}", name))
        })
        .collect();

    let app_children: Vec<(String, String)> = apps_registry
        .iter()
        .map(|(name, meta)| {
            let label = meta
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or(name)
                .to_string();
            (label, format!("app:{}", name))
        })
        .collect();

    vec![
        NavItem {
            label: "Dashboard".to_string(),
            content: NavContent::Children(dash_children),
        },
        NavItem {
            label: "Apps".to_string(),
            content: NavContent::Children(app_children),
        },
        NavItem {
            label: "Settings".to_string(),
            content: NavContent::Direct("page:settings".to_string()),
        },
        NavItem {
            label: "Touchscreen".to_string(),
            content: NavContent::Direct("page:touchscreen".to_string()),
        },
        NavItem {
            label: "System".to_string(),
            content: NavContent::Children(vec![
                ("Task Manager".to_string(), "page:taskmanager".to_string()),
                ("Logs".to_string(), "page:logs".to_string()),
                ("Restart".to_string(), "system:restart".to_string()),
                ("Shutdown".to_string(), "system:shutdown".to_string()),
            ]),
        },
    ]
}

// ---------------------------------------------------------------------------
// Custom navbar renderer
// ---------------------------------------------------------------------------

fn render_navbar(frame: &mut Frame, area: Rect, nav_items: &[NavItem], state: &TuixState) {
    let focused = state.focus == FocusTarget::Navbar;
    let labels: Vec<&str> = nav_items.iter().map(|i| i.label.as_str()).collect();

    let mut constraints: Vec<Constraint> = labels
        .iter()
        .map(|lbl| Constraint::Length((lbl.len() + 4) as u16))
        .collect();
    constraints.push(Constraint::Fill(1));

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (i, label) in labels.iter().enumerate() {
        let style = if focused && i == state.nav_cursor {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };
        frame.render_widget(
            Paragraph::new(Text::raw(format!("  {}  ", label))).style(style),
            cols[i],
        );
    }
}

fn render_dropdown(frame: &mut Frame, area: Rect, nav_items: &[NavItem], state: &TuixState) {
    if !state.nav_expanded {
        return;
    }

    let item = &nav_items[state.nav_cursor];
    let children = match &item.content {
        NavContent::Children(c) => c,
        NavContent::Direct(_) => return,
    };

    if children.is_empty() {
        return;
    }

    // Calculate x offset below the nav item
    let labels: Vec<&str> = nav_items.iter().map(|i| i.label.as_str()).collect();
    let x_offset: u16 = labels[..state.nav_cursor]
        .iter()
        .map(|lbl| (lbl.len() + 4) as u16)
        .sum();

    let max_label_len = children.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
    let mut dropdown_width = (max_label_len + 6) as u16;
    let dropdown_height = (children.len() + 2) as u16;

    let available_width = area.width.saturating_sub(x_offset);
    if dropdown_width > available_width {
        dropdown_width = available_width.max(10);
    }

    let dropdown_rect = Rect {
        x: area.x + x_offset,
        y: area.y,
        width: dropdown_width,
        height: dropdown_height.min(area.height),
    };

    // Clear the area and draw border
    frame.render_widget(Clear, dropdown_rect);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black)),
        dropdown_rect,
    );

    let inner = dropdown_rect.inner(Margin { horizontal: 1, vertical: 1 });

    let row_constraints: Vec<Constraint> = children
        .iter()
        .map(|_| Constraint::Length(1))
        .collect();

    let inner_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(inner);

    for (i, (label, _)) in children.iter().enumerate() {
        let (style, text) = if i == state.dropdown_cursor {
            (
                Style::default().fg(Color::Black).bg(Color::Cyan),
                format!(" » {} ", label),
            )
        } else {
            (
                Style::default().fg(Color::White).bg(Color::Black),
                format!("   {} ", label),
            )
        };
        if i < inner_rows.len() {
            frame.render_widget(
                Paragraph::new(Text::raw(text)).style(style),
                inner_rows[i],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Status colors for system page
// ---------------------------------------------------------------------------

fn status_color(status: AppStatus) -> Color {
    match status {
        AppStatus::Running => Color::Green,
        AppStatus::Idle => Color::Yellow,
        AppStatus::Stopped => Color::DarkGray,
    }
}

// ---------------------------------------------------------------------------
// System page renderer
// ---------------------------------------------------------------------------

fn render_system_page(frame: &mut Frame, area: Rect, state: &TuixState, border_style: Style) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Task Manager — Running Processes ")
        .title_alignment(ratatui::layout::Alignment::Right)
        .style(border_style);
    frame.render_widget(block, area);
    let inner = area.inner(Margin { horizontal: 1, vertical: 1 });

    let running = process_manager::list_running();
    if running.is_empty() {
        frame.render_widget(
            Paragraph::new(Text::raw(
                "\n  No applications are currently running.\n\n  Press Q to return.",
            ))
            .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let mut row_constraints = vec![Constraint::Length(1), Constraint::Length(1)];
    for _ in &running {
        row_constraints.push(Constraint::Length(1));
    }
    row_constraints.push(Constraint::Length(2));

    let header_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(inner);

    frame.render_widget(
        Paragraph::new(Text::raw(format!(
            "  {:<20} {:<8} {:<10} {}",
            "NAME", "PID", "STATUS", "STARTED"
        )))
        .style(Style::default().fg(Color::DarkGray)),
        header_rows[0],
    );
    frame.render_widget(
        Paragraph::new(Text::raw(format!("  {}", "─".repeat(60))))
            .style(Style::default().fg(Color::DarkGray)),
        header_rows[1],
    );

    for (i, app) in running.iter().enumerate() {
        let started = app.start_time.format("%H:%M:%S").to_string();
        let name_display = if i == state.system_cursor {
            format!("» {}", app.name)
        } else {
            format!("  {}", app.name)
        };
        let row_text = format!(
            "  {:<20} {:<8} {:<10} {}",
            name_display,
            app.pid.map(|p| p.to_string()).unwrap_or_default(),
            format!("{:?}", app.status),
            started
        );
        let style = if i == state.system_cursor {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default().fg(status_color(app.status))
        };
        frame.render_widget(
            Paragraph::new(Text::raw(row_text)).style(style),
            header_rows[2 + i],
        );
    }

    frame.render_widget(
        Paragraph::new(Text::raw(
            "\n  Enter — stop selected app   Q — return",
        ))
        .style(Style::default().fg(Color::DarkGray)),
        header_rows[2 + running.len()],
    );
}

// ---------------------------------------------------------------------------
// App log tail renderer
// ---------------------------------------------------------------------------

fn render_app_log(frame: &mut Frame, area: Rect, app_name: &str, border_style: Style) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} — Log Output ", app_name))
        .title_alignment(ratatui::layout::Alignment::Right)
        .style(border_style);
    frame.render_widget(block, area);
    let inner = area.inner(Margin { horizontal: 1, vertical: 1 });

    let n = if inner.height > 0 { inner.height as usize } else { 20 };
    let lines = process_manager::log_tail(app_name, n);
    let content = if lines.is_empty() {
        "  (no output yet — waiting for subprocess...)".to_string()
    } else {
        lines.iter().map(|l| format!("  {}", l)).collect::<Vec<_>>().join("\n")
    };

    frame.render_widget(
        Paragraph::new(Text::raw(content)).style(Style::default().fg(Color::White)),
        inner,
    );
}

// ---------------------------------------------------------------------------
// Logs page renderer — shows tuix.log with colored severity levels
// ---------------------------------------------------------------------------

fn render_logs_page(frame: &mut Frame, area: Rect, state: &TuixState, border_style: Style) {
    use ratatui::text::{Line, Span};

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Logs — tuix.log ")
        .title_alignment(ratatui::layout::Alignment::Right)
        .style(border_style);
    frame.render_widget(block, area);
    let inner = area.inner(Margin { horizontal: 1, vertical: 1 });

    let lines = logging::read_log_lines();

    if lines.is_empty() {
        frame.render_widget(
            Paragraph::new(Text::raw("\n  No log entries yet."))
                .style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    // log_scroll is a reverse offset from the bottom (0 = most recent entries).
    // Compute the start line so that the view ends at (total - log_scroll).
    let total = lines.len();
    let visible_height = inner.height as usize;
    let max_offset = total.saturating_sub(visible_height);
    let offset = state.log_scroll.min(max_offset);
    let start = max_offset.saturating_sub(offset);
    let end = (start + visible_height).min(total);
    let visible_lines = &lines[start..end];

    let styled_lines: Vec<Line> = visible_lines
        .iter()
        .map(|line| {
            // Find severity tag and color just the tag text
            if let Some(start) = line.find("[INFO]") {
                let before = &line[..start];
                let tag = "[INFO]";
                let after = &line[start + tag.len()..];
                Line::from(vec![
                    Span::styled(format!("  {}", before), Style::default().fg(Color::White)),
                    Span::styled(tag, Style::default().fg(Color::Cyan)),
                    Span::styled(after.to_string(), Style::default().fg(Color::White)),
                ])
            } else if let Some(start) = line.find("[WARN]") {
                let before = &line[..start];
                let tag = "[WARN]";
                let after = &line[start + tag.len()..];
                Line::from(vec![
                    Span::styled(format!("  {}", before), Style::default().fg(Color::White)),
                    Span::styled(tag, Style::default().fg(Color::Yellow)),
                    Span::styled(after.to_string(), Style::default().fg(Color::White)),
                ])
            } else if let Some(start) = line.find("[ERROR]") {
                let before = &line[..start];
                let tag = "[ERROR]";
                let after = &line[start + tag.len()..];
                Line::from(vec![
                    Span::styled(format!("  {}", before), Style::default().fg(Color::White)),
                    Span::styled(tag, Style::default().fg(Color::Red)),
                    Span::styled(after.to_string(), Style::default().fg(Color::White)),
                ])
            } else if let Some(start) = line.find("[SCRIPT]") {
                let before = &line[..start];
                let tag = "[SCRIPT]";
                let after = &line[start + tag.len()..];
                Line::from(vec![
                    Span::styled(format!("  {}", before), Style::default().fg(Color::White)),
                    Span::styled(tag, Style::default().fg(Color::Magenta)),
                    Span::styled(after.to_string(), Style::default().fg(Color::White)),
                ])
            } else {
                Line::from(Span::styled(
                    format!("  {}", line),
                    Style::default().fg(Color::DarkGray),
                ))
            }
        })
        .collect();

    let scrollbar_info = format!(
        " Line {}-{} of {} ",
        start + 1,
        end,
        total
    );

    frame.render_widget(
        Paragraph::new(styled_lines),
        inner,
    );

    // Render scroll position indicator at bottom-right of border
    let info_width = scrollbar_info.len() as u16;
    if area.width > info_width + 2 {
        let info_rect = Rect {
            x: area.x + area.width - info_width - 1,
            y: area.y + area.height - 1,
            width: info_width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Text::raw(scrollbar_info))
                .style(Style::default().fg(Color::DarkGray)),
            info_rect,
        );
    }
}

// ---------------------------------------------------------------------------
// Main container renderer
// ---------------------------------------------------------------------------

fn render_main(
    frame: &mut Frame,
    area: Rect,
    state: &TuixState,
    active_internal_app: &mut Option<InternalApp>,
    active_installed_dash: &mut Option<InstalledDashboard>,
) {
    let focused = state.focus == FocusTarget::Main;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    if let Some(page) = &state.active_page {
        match page.as_str() {
            "settings" => settings::page::render(frame, area, border_style),
            "touchscreen" => touchscreen::page::render(frame, area, border_style),
            "system" | "taskmanager" => render_system_page(frame, area, state, border_style),
            "logs" => render_logs_page(frame, area, state, border_style),
            _ => {}
        }
    } else if state.active_app.is_some() {
        if let Some(app) = active_internal_app {
            app.render(frame, area, border_style);
        } else if let Some(app_name) = &state.active_app {
            render_app_log(frame, area, app_name, border_style);
        }
    } else if let Some(dash) = active_installed_dash {
        if state.active_dashboard == dash.name {
            dash.render(frame, area, border_style);
        } else {
            // Dashboard changed away from installed one
            match state.active_dashboard.as_str() {
                "Dashboard-1" => dashboard_1::render(frame, area, border_style),
                "Dashboard-2" => dashboard_2::render(frame, area, border_style),
                _ => render_no_dashboard(frame, area, border_style),
            }
        }
    } else {
        match state.active_dashboard.as_str() {
            "Dashboard-1" => dashboard_1::render(frame, area, border_style),
            "Dashboard-2" => dashboard_2::render(frame, area, border_style),
            _ => render_no_dashboard(frame, area, border_style),
        }
    }
}

fn render_no_dashboard(frame: &mut Frame, area: Rect, border_style: Style) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" TUIX ")
        .style(border_style);
    frame.render_widget(block, area);
    let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
    frame.render_widget(
        Paragraph::new(Text::raw("\n  No dashboard loaded."))
            .style(Style::default().fg(Color::DarkGray)),
        inner,
    );
}

// ---------------------------------------------------------------------------
// Navbar key handling
// ---------------------------------------------------------------------------

enum NavResult {
    None,
    Quit,
    Restart,
    ActivateInternalApp,
    #[allow(dead_code)]
    RunForeground(Vec<String>),
}

fn handle_navbar_key(
    action: Action,
    state: &mut TuixState,
    nav_items: &[NavItem],
    apps_registry: &std::collections::HashMap<String, Value>,
    dashboards_registry: &std::collections::HashMap<String, Value>,
    active_internal_app: &mut Option<InternalApp>,
    active_installed_dash: &mut Option<InstalledDashboard>,
) -> NavResult {
    if action == Action::Tab {
        state.focus = FocusTarget::Main;
        state.nav_expanded = false;
        return NavResult::None;
    }

    if action == Action::Quit {
        return NavResult::Quit;
    }

    if action == Action::Back {
        if state.nav_expanded {
            state.nav_expanded = false;
            state.dropdown_cursor = 0;
        }
        return NavResult::None;
    }

    // --- Dropdown is open ---
    if state.nav_expanded {
        let item = &nav_items[state.nav_cursor];
        if let NavContent::Children(children) = &item.content {
            match action {
                Action::Up => {
                    if state.dropdown_cursor > 0 {
                        state.dropdown_cursor -= 1;
                    }
                }
                Action::Down => {
                    if state.dropdown_cursor < children.len().saturating_sub(1) {
                        state.dropdown_cursor += 1;
                    }
                }
                Action::Enter => {
                    if !children.is_empty() {
                        let data = children[state.dropdown_cursor].1.clone();
                        return execute_action(
                            &data,
                            state,
                            apps_registry,
                            dashboards_registry,
                            active_internal_app,
                            active_installed_dash,
                        );
                    }
                }
                _ => {}
            }
        }
        return NavResult::None;
    }

    // --- Dropdown is closed (top-level navigation) ---
    match action {
        Action::Left => {
            if state.nav_cursor == 0 {
                state.nav_cursor = nav_items.len() - 1;
            } else {
                state.nav_cursor -= 1;
            }
        }
        Action::Right => {
            state.nav_cursor = (state.nav_cursor + 1) % nav_items.len();
        }
        Action::Enter | Action::Down => {
            let item = &nav_items[state.nav_cursor];
            match &item.content {
                NavContent::Children(_) => {
                    state.nav_expanded = true;
                    state.dropdown_cursor = 0;
                }
                NavContent::Direct(data) => {
                    let data = data.clone();
                    return execute_action(
                        &data,
                        state,
                        apps_registry,
                        dashboards_registry,
                        active_internal_app,
                        active_installed_dash,
                    );
                }
            }
        }
        _ => {}
    }

    NavResult::None
}

fn execute_action(
    data: &str,
    state: &mut TuixState,
    apps_registry: &std::collections::HashMap<String, Value>,
    dashboards_registry: &std::collections::HashMap<String, Value>,
    active_internal_app: &mut Option<InternalApp>,
    active_installed_dash: &mut Option<InstalledDashboard>,
) -> NavResult {
    state.nav_expanded = false;
    state.dropdown_cursor = 0;

    if let Some(name) = data.strip_prefix("dashboard:") {
        logging::info(&format!("Navigated to dashboard: {}", name));
        state.active_dashboard = name.to_string();
        state.active_page = None;
        state.active_app = None;
        state.focus = FocusTarget::Main;

        // Check if this is an installed (third-party) dashboard
        if let Some(meta) = dashboards_registry.get(name) {
            let dash_type = meta.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if dash_type == "installed" {
                let source = meta.get("source").and_then(|s| s.as_str()).unwrap_or("global");
                let cmd: Vec<String> = if source == "local" {
                    // Run from local downloads path
                    let local_bin = format!("downloads/dashboards/{}/target/release/{}", name, name);
                    let local_path = std::path::Path::new(&local_bin);
                    if local_path.exists() {
                        vec![local_bin]
                    } else {
                        logging::error(&format!("Local binary not found for dashboard {}: {}", name, local_bin));
                        vec![]
                    }
                } else {
                    // Run from global PATH (cargo install puts it in ~/.cargo/bin/)
                    meta.get("cmd")
                        .and_then(|c| c.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default()
                };

                let label = meta
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or(name)
                    .to_string();

                // Kill previous installed dashboard if any
                *active_installed_dash = None;

                // Spawn new one (use default size, will resize on first render)
                if !cmd.is_empty() {
                    *active_installed_dash =
                        InstalledDashboard::start(name, &label, &cmd, 80, 24);
                }
            } else {
                // Switching to a built-in dashboard; drop any installed one
                *active_installed_dash = None;
            }
        } else {
            *active_installed_dash = None;
        }
    } else if let Some(name) = data.strip_prefix("app:") {
        logging::info(&format!("Opened app: {}", name));
        let meta = apps_registry.get(name);
        let app_type = meta
            .and_then(|m| m.get("type"))
            .and_then(|t| t.as_str())
            .unwrap_or("external");

        if app_type == "internal" {
            let internal = match name {
                "TextEditor" => {
                    let mut app = TextEditorApp::new();
                    app.start();
                    InternalApp::TextEditor(app)
                }
                "FileExplorer" => {
                    let mut app = FileExplorerApp::new();
                    app.start();
                    InternalApp::FileExplorer(app)
                }
                _ => {
                    let mut app = CharacterSetApp::new();
                    app.start();
                    InternalApp::CharacterSet(app)
                }
            };
            *active_internal_app = Some(internal);
            state.active_app = Some(name.to_string());
            state.active_page = None;
            state.focus = FocusTarget::Main;
            return NavResult::ActivateInternalApp;
        } else {
            let cmd: Vec<String> = meta
                .and_then(|m| m.get("cmd"))
                .and_then(|c| c.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            if !cmd.is_empty() {
                process_manager::launch(name, &cmd);
            }
            state.active_app = Some(name.to_string());
            state.active_page = None;
            state.focus = FocusTarget::Main;
        }
    } else if let Some(page_name) = data.strip_prefix("page:") {
        logging::info(&format!("Opened page: {}", page_name));
        if page_name == "logs" {
            state.log_scroll = 0; // 0 = show bottom (most recent entries)
        }
        state.active_page = Some(page_name.to_string());
        state.active_app = None;
        state.focus = FocusTarget::Main;
    } else if data == "system:shutdown" {
        logging::info("System shutdown requested");
        return NavResult::Quit;
    } else if data == "system:restart" {
        logging::info("System restart requested");
        return NavResult::Restart;
    }

    NavResult::None
}

// ---------------------------------------------------------------------------
// Main container key handling
// ---------------------------------------------------------------------------

enum MainResult {
    None,
    Quit,
    #[allow(dead_code)]
    RunForeground(Vec<String>),
}

fn handle_main_key(
    action: Action,
    state: &mut TuixState,
    active_internal_app: &mut Option<InternalApp>,
    active_installed_dash: &mut Option<InstalledDashboard>,
) -> MainResult {
    if action == Action::Quit {
        return MainResult::Quit;
    }

    if action == Action::Tab {
        state.focus = FocusTarget::Navbar;
        return MainResult::None;
    }

    // If an internal app has its OSK active, forward ALL mapped keys to it
    // (including Back/Q which normally closes the app — the OSK handles
    // Q as "close keyboard", not "close app").
    if let Some(app) = active_internal_app {
        if app.wants_raw_wasd() {
            if let Some(key_name) = action_to_key_name(action) {
                if app.handle_key(key_name) {
                    app.stop();
                    state.active_app = None;
                    *active_internal_app = None;
                }
            }
            return MainResult::None;
        }
    }

    if action == Action::Back {
        if state.active_page.is_some() {
            state.active_page = None;
            state.active_app = None;
        } else if state.active_app.is_some() {
            if let Some(app) = active_internal_app {
                app.stop();
            }
            state.active_app = None;
            *active_internal_app = None;
        } else if active_installed_dash.is_some() {
            // Close the installed dashboard and go back to default
            *active_installed_dash = None;
            state.active_dashboard = "Dashboard-1".to_string();
        }
        return MainResult::None;
    }

    // Forward keys to installed dashboard if active
    if let Some(dash) = active_installed_dash {
        if state.active_dashboard == dash.name && state.active_page.is_none() && state.active_app.is_none() {
            dash.send_key(action);
            return MainResult::None;
        }
    }

    // System page navigation
    if matches!(state.active_page.as_deref(), Some("system") | Some("taskmanager")) {
        let running = process_manager::list_running();
        match action {
            Action::Up => {
                if state.system_cursor > 0 {
                    state.system_cursor -= 1;
                }
            }
            Action::Down => {
                let max_idx = running.len().saturating_sub(1);
                if state.system_cursor < max_idx {
                    state.system_cursor += 1;
                }
            }
            Action::Enter => {
                if !running.is_empty() {
                    let idx = state.system_cursor.min(running.len() - 1);
                    process_manager::stop(&running[idx].name);
                    state.system_cursor = 0;
                }
            }
            _ => {}
        }
        return MainResult::None;
    }

    // Logs page navigation
    if matches!(state.active_page.as_deref(), Some("logs")) {
        let total_lines = logging::read_log_lines().len();
        match action {
            // Up = scroll toward older entries (increase reverse offset from bottom)
            Action::Up => {
                let max_offset = total_lines.saturating_sub(1);
                state.log_scroll = (state.log_scroll + 1).min(max_offset);
            }
            // Down = scroll toward newer entries (decrease reverse offset)
            Action::Down => {
                state.log_scroll = state.log_scroll.saturating_sub(1);
            }
            _ => {}
        }
        return MainResult::None;
    }

    // Internal app key forwarding
    if let Some(app) = active_internal_app {
        if let Some(key_name) = action_to_key_name(action) {
            if app.handle_key(key_name) {
                app.stop();
                state.active_app = None;
                *active_internal_app = None;
            }
        }
    }

    MainResult::None
}

// ---------------------------------------------------------------------------
// Restart is handled by returning from the main loop and re-entering it.
// No exec or spawn needed — clean terminal restore between cycles.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Foreground subprocess execution (takes over terminal)
// ---------------------------------------------------------------------------

fn run_foreground(cmd: &[String]) {
    if cmd.is_empty() {
        return;
    }
    let _ = Command::new(&cmd[0]).args(&cmd[1..]).status();
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn main() {
    loop {
        let should_restart = run_app();
        if !should_restart {
            break;
        }
    }
}

/// Run the TUIX app. Returns `true` if the user requested a restart.
fn run_app() -> bool {
    std::fs::create_dir_all("logs").ok();

    logging::init();
    logging::info("TUIX started");

    let settings_map = registry::load_settings();
    let dashboards = registry::load_dashboards();
    let apps_registry = registry::load_apps();

    let default_dashboard = settings_map
        .get("default_dashboard")
        .and_then(|v| v.as_str())
        .unwrap_or("Dashboard-1")
        .to_string();

    let mut state = TuixState::new(default_dashboard);
    let nav_items = build_nav_items(&dashboards, &apps_registry);
    let mut active_internal_app: Option<InternalApp> = None;
    let mut active_installed_dash: Option<InstalledDashboard> = None;

    let mut should_restart = false;

    // Setup terminal
    enable_raw_mode().expect("Failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).expect("Failed to enter alternate screen");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    loop {
        // Draw UI
        terminal
            .draw(|frame| {
                let rows = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Fill(1)])
                    .split(frame.area());

                // 1. Draw main container
                render_main(frame, rows[1], &state, &mut active_internal_app, &mut active_installed_dash);

                // 2. Draw navbar label row
                render_navbar(frame, rows[0], &nav_items, &state);

                // 3. Draw dropdown LAST (overlays main container)
                if state.nav_expanded {
                    render_dropdown(frame, rows[1], &nav_items, &state);
                }
            })
            .expect("Failed to draw frame");

        // Poll for events with timeout
        if !event::poll(Duration::from_millis(100)).unwrap_or(false) {
            continue;
        }

        let ev = match event::read() {
            Ok(ev) => ev,
            Err(_) => continue,
        };

        // Only handle key press events (not release/repeat)
        let key_event = match ev {
            Event::Key(key) if key.kind == KeyEventKind::Press => key,
            _ => continue,
        };

        // When an installed dashboard is active and main is focused,
        // forward raw key events directly to the PTY subprocess.
        // Only Esc (quit) and Tab (switch to navbar) are reserved by TUIX.
        if state.focus == FocusTarget::Main {
            if let Some(dash) = &mut active_installed_dash {
                if state.active_dashboard == dash.name
                    && state.active_page.is_none()
                    && state.active_app.is_none()
                {
                    match key_event.code {
                        crossterm::event::KeyCode::Esc => break, // Quit TUIX
                        crossterm::event::KeyCode::Tab => {
                            state.focus = FocusTarget::Navbar;
                            continue;
                        }
                        _ => {
                            dash.send_key_event(key_event);
                            continue;
                        }
                    }
                }
            }
        }

        let action = match map_key(key_event) {
            Some(a) => a,
            None => continue,
        };

        if action == Action::Quit {
            break;
        }

        if state.focus == FocusTarget::Navbar {
            match handle_navbar_key(
                action,
                &mut state,
                &nav_items,
                &apps_registry,
                &dashboards,
                &mut active_internal_app,
                &mut active_installed_dash,
            ) {
                NavResult::Quit => break,
                NavResult::Restart => {
                    should_restart = true;
                    break;
                }
                NavResult::RunForeground(cmd) => {
                    // Restore terminal, run subprocess, re-enter
                    disable_raw_mode().ok();
                    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
                    run_foreground(&cmd);
                    enable_raw_mode().ok();
                    execute!(terminal.backend_mut(), EnterAlternateScreen).ok();
                    terminal.clear().ok();
                }
                NavResult::ActivateInternalApp | NavResult::None => {}
            }
        } else {
            match handle_main_key(action, &mut state, &mut active_internal_app, &mut active_installed_dash) {
                MainResult::Quit => break,
                MainResult::RunForeground(cmd) => {
                    disable_raw_mode().ok();
                    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
                    run_foreground(&cmd);
                    enable_raw_mode().ok();
                    execute!(terminal.backend_mut(), EnterAlternateScreen).ok();
                    terminal.clear().ok();
                }
                MainResult::None => {}
            }
            if state.active_app.is_none() && active_internal_app.is_some() {
                active_internal_app = None;
            }
        }
    }

    // Drop installed dashboard PTY before restoring terminal
    drop(active_installed_dash);
    drop(active_internal_app);

    // Restore terminal
    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    drop(terminal);

    should_restart
}
