/// App Store state — enums and structs for the App Store UI.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    AtoZ,
    Author,
    Category,
}

impl SortMode {
    pub const ALL: &'static [SortMode] = &[
        SortMode::AtoZ,
        SortMode::Author,
        SortMode::Category,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            SortMode::AtoZ => "A to Z",
            SortMode::Author => "Author",
            SortMode::Category => "Category",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallLocation {
    Global,
    Local,
}

impl InstallLocation {
    pub fn label(&self) -> &'static str {
        match self {
            InstallLocation::Global => "PATH (cargo install)",
            InstallLocation::Local => "Downloads (git clone + build)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallStatus {
    NotInstalled,
    Global(String),
    Local(String),
    Both(String, String),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ConfirmAction {
    UninstallGlobal(String),
    UninstallLocal(String),
    UninstallChoose(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AppStoreFocus {
    LeftPane,
    RightPane,
    SearchBar,
    SortDropdown,
    FilterDropdown,
    ConfirmDialog,
    InstallLocationDialog,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AppStoreState {
    pub focus: AppStoreFocus,
    pub left_cursor: usize,
    pub left_scroll: usize,
    pub right_scroll: usize,
    pub sort_mode: SortMode,
    pub sort_dropdown_open: bool,
    pub sort_dropdown_cursor: usize,
    pub filter_show_installed: bool,
    pub filter_show_uninstalled: bool,
    pub filter_categories: HashSet<String>,
    pub filter_dropdown_open: bool,
    pub filter_dropdown_cursor: usize,
    pub search_query: String,
    pub search_active: bool,
    pub terminal_output: Vec<String>,
    pub terminal_focused: bool,
    pub terminal_scroll: usize,
    pub terminal_visible: bool,
    pub operation_running: bool,
    pub confirm_dialog: Option<ConfirmAction>,
    pub confirm_cursor: usize,
    pub install_location_dialog: bool,
    pub install_location_cursor: usize,
    pub available_install_methods: Vec<InstallLocation>,
    pub computed_app_list: Vec<String>,
    pub install_statuses: HashMap<String, InstallStatus>,
    pub right_action_cursor: usize,
    pub in_right_actions: bool,
    /// Shared output buffer for background install/uninstall thread
    pub thread_output: Arc<Mutex<Vec<String>>>,
    /// Signals that the background operation has completed
    pub thread_done: Arc<AtomicBool>,
    /// Signals whether the operation succeeded
    pub thread_success: Arc<AtomicBool>,
    /// The install location used for the current operation (for post-install config update)
    pub pending_install_location: Option<InstallLocation>,
    /// The app key for the current background operation
    pub pending_op_key: Option<String>,
    /// Whether the pending operation is an install (true) or uninstall (false)
    pub pending_is_install: bool,
}

impl AppStoreState {
    pub fn new() -> Self {
        Self {
            focus: AppStoreFocus::LeftPane,
            left_cursor: 0,
            left_scroll: 0,
            right_scroll: 0,
            sort_mode: SortMode::AtoZ,
            sort_dropdown_open: false,
            sort_dropdown_cursor: 0,
            filter_show_installed: true,
            filter_show_uninstalled: true,
            filter_categories: HashSet::new(),
            filter_dropdown_open: false,
            filter_dropdown_cursor: 0,
            search_query: String::new(),
            search_active: false,
            terminal_output: Vec::new(),
            terminal_focused: false,
            terminal_scroll: 0,
            terminal_visible: false,
            operation_running: false,
            confirm_dialog: None,
            confirm_cursor: 0,
            install_location_dialog: false,
            install_location_cursor: 0,
            available_install_methods: Vec::new(),
            computed_app_list: Vec::new(),
            install_statuses: HashMap::new(),
            right_action_cursor: 0,
            in_right_actions: false,
            thread_output: Arc::new(Mutex::new(Vec::new())),
            thread_done: Arc::new(AtomicBool::new(false)),
            thread_success: Arc::new(AtomicBool::new(false)),
            pending_install_location: None,
            pending_op_key: None,
            pending_is_install: false,
        }
    }

    pub fn selected_app_key(&self) -> Option<&String> {
        self.computed_app_list.get(self.left_cursor)
    }

    /// Sync output from background thread and check completion.
    /// Returns true if the operation just completed.
    pub fn poll_operation(&mut self) -> bool {
        if !self.operation_running {
            return false;
        }
        // Sync output from thread
        if let Ok(mut buf) = self.thread_output.try_lock() {
            if !buf.is_empty() {
                self.terminal_output.append(&mut *buf);
                // Auto-scroll to bottom
                let visible = 10usize; // approximate
                self.terminal_scroll = self.terminal_output.len().saturating_sub(visible);
            }
        }
        // Check completion
        if self.thread_done.load(Ordering::Relaxed) {
            self.operation_running = false;
            // Final sync
            if let Ok(mut buf) = self.thread_output.try_lock() {
                self.terminal_output.append(&mut *buf);
            }
            return true;
        }
        false
    }

    /// Start a new background operation (resets shared state).
    pub fn start_operation(&mut self, key: String, is_install: bool, location: Option<InstallLocation>) {
        self.terminal_output.clear();
        self.terminal_visible = true;
        self.operation_running = true;
        self.pending_op_key = Some(key);
        self.pending_is_install = is_install;
        self.pending_install_location = location;
        self.thread_output = Arc::new(Mutex::new(Vec::new()));
        self.thread_done = Arc::new(AtomicBool::new(false));
        self.thread_success = Arc::new(AtomicBool::new(false));
    }

    pub fn reset_right_pane(&mut self) {
        self.right_scroll = 0;
        self.right_action_cursor = 0;
        self.in_right_actions = false;
        if !self.operation_running {
            self.terminal_visible = false;
            self.terminal_output.clear();
            self.terminal_focused = false;
            self.terminal_scroll = 0;
        }
    }

    /// Build filtered and sorted list from registered apps.
    pub fn recompute_app_list(&mut self, registered: &HashMap<String, Value>) {
        let mut keys: Vec<String> = registered
            .keys()
            .filter(|key| {
                let meta = &registered[key.as_str()];

                // Search filter (name only)
                if !self.search_query.is_empty() {
                    let label = meta.get("label").and_then(|v| v.as_str()).unwrap_or(key);
                    let query_lower = self.search_query.to_lowercase();
                    if !label.to_lowercase().contains(&query_lower)
                        && !key.to_lowercase().contains(&query_lower)
                    {
                        return false;
                    }
                }

                // Installed / uninstalled filter
                let is_installed = matches!(
                    self.install_statuses.get(key.as_str()),
                    Some(InstallStatus::Global(_))
                        | Some(InstallStatus::Local(_))
                        | Some(InstallStatus::Both(_, _))
                );
                if is_installed && !self.filter_show_installed {
                    return false;
                }
                if !is_installed && !self.filter_show_uninstalled {
                    return false;
                }

                // Category filter (set contains hidden categories)
                if !self.filter_categories.is_empty() {
                    let cat = meta
                        .get("category")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Other")
                        .to_string();
                    if self.filter_categories.contains(&cat) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort
        keys.sort_by(|a, b| {
            let ma = &registered[a.as_str()];
            let mb = &registered[b.as_str()];
            match self.sort_mode {
                SortMode::AtoZ => {
                    let la = ma.get("label").and_then(|v| v.as_str()).unwrap_or(a);
                    let lb = mb.get("label").and_then(|v| v.as_str()).unwrap_or(b);
                    la.to_lowercase().cmp(&lb.to_lowercase())
                }
                SortMode::Author => {
                    let aa = ma.get("author").and_then(|v| v.as_str()).unwrap_or("");
                    let ab = mb.get("author").and_then(|v| v.as_str()).unwrap_or("");
                    aa.to_lowercase().cmp(&ab.to_lowercase())
                }
                SortMode::Category => {
                    let ca = ma.get("category").and_then(|v| v.as_str()).unwrap_or("");
                    let cb = mb.get("category").and_then(|v| v.as_str()).unwrap_or("");
                    ca.to_lowercase()
                        .cmp(&cb.to_lowercase())
                        .then_with(|| {
                            let la = ma.get("label").and_then(|v| v.as_str()).unwrap_or(a);
                            let lb = mb.get("label").and_then(|v| v.as_str()).unwrap_or(b);
                            la.to_lowercase().cmp(&lb.to_lowercase())
                        })
                }
            }
        });

        self.computed_app_list = keys;

        // Clamp cursor
        if !self.computed_app_list.is_empty() {
            self.left_cursor = self.left_cursor.min(self.computed_app_list.len() - 1);
        } else {
            self.left_cursor = 0;
        }
    }

    /// Collect all unique categories from registered apps.
    pub fn all_categories(registered: &HashMap<String, Value>) -> Vec<String> {
        let mut cats: Vec<String> = registered
            .values()
            .filter_map(|v| v.get("category").and_then(|c| c.as_str()))
            .map(|s| s.to_string())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        cats.sort();
        cats
    }
}
