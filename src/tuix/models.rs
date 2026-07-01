/// TUIX — shared enums, structs, and state containers.

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Back,  // Q / Backspace
    Tab,
    Quit,  // Esc
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    Navbar,
    Main,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AppStatus {
    Running,
    Idle,
    Stopped,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RunningApp {
    pub name: String,
    pub pid: Option<u32>,
    pub status: AppStatus,
    pub start_time: DateTime<Utc>,
    pub log_path: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TuixState {
    pub focus: FocusTarget,
    pub active_dashboard: String,
    /// "settings" | "touchscreen" | "taskmanager" | None
    pub active_page: Option<String>,
    /// name of an active internal app, or None
    pub active_app: Option<String>,
    pub running_apps: Vec<RunningApp>,
    /// index highlighted in the Task Manager process list
    pub system_cursor: usize,
    /// Custom navbar state
    pub nav_cursor: usize,
    pub nav_expanded: bool,
    pub dropdown_cursor: usize,
    /// Submenu state (e.g. Branches flyout from System dropdown)
    pub submenu_open: bool,
    pub submenu_cursor: usize,
    /// Cached submenu items: (label, action_string)
    pub submenu_items: Vec<(String, String)>,
}

impl TuixState {
    pub fn new(default_dashboard: String) -> Self {
        Self {
            focus: FocusTarget::Navbar,
            active_dashboard: default_dashboard,
            active_page: None,
            active_app: None,
            running_apps: Vec::new(),
            system_cursor: 0,
            nav_cursor: 0,
            nav_expanded: false,
            dropdown_cursor: 0,
            submenu_open: false,
            submenu_cursor: 0,
            submenu_items: Vec::new(),
        }
    }
}
