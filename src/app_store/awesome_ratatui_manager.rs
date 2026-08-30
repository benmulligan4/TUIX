/// Awesome Ratatui browser — fetch, parse, cache the awesome-ratatui README,
/// and provide state/rendering for browsing and adding apps.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::utilities::logging;

const README_URL: &str =
    "https://raw.githubusercontent.com/ratatui/awesome-ratatui/main/README.md";
const CACHE_FILE: &str = "awesome-ratatui-cache.json";
const CACHE_TTL_SECS: u64 = 3600;

// ── Data types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwesomeApp {
    pub name: String,
    pub repo_url: String,
    pub description: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Cache {
    fetched_at: u64,
    apps: Vec<AwesomeApp>,
}

// ── Browser state ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserFocus {
    SearchBar,
    DescriptionToggle,
    List,
    Actions,
}

#[derive(Debug, Clone)]
pub struct BrowserState {
    pub active: bool,
    pub focus: BrowserFocus,
    pub apps: Vec<AwesomeApp>,
    pub categories: Vec<String>,
    pub collapsed: std::collections::HashSet<String>,
    pub search_query: String,
    pub cursor: usize,
    pub scroll: usize,
    /// Visible rows after filtering/collapsing (category headers + apps)
    pub visible_rows: Vec<RowKind>,
    pub fetched_at: Option<u64>,
    pub action_cursor: usize,
    pub loading: bool,
    pub error: Option<String>,
    pub show_descriptions: bool,
}

#[derive(Debug, Clone)]
pub enum RowKind {
    CategoryHeader(String),
    App(usize),
    Spacer,
}

impl BrowserState {
    pub fn new() -> Self {
        Self {
            active: false,
            focus: BrowserFocus::List,
            apps: Vec::new(),
            categories: Vec::new(),
            collapsed: std::collections::HashSet::new(),
            search_query: String::new(),
            cursor: 0,
            scroll: 0,
            visible_rows: Vec::new(),
            fetched_at: None,
            action_cursor: 0,
            loading: false,
            error: None,
            show_descriptions: true,
        }
    }

    pub fn selected_app(&self) -> Option<&AwesomeApp> {
        if let Some(RowKind::App(idx)) = self.visible_rows.get(self.cursor) {
            self.apps.get(*idx)
        } else {
            None
        }
    }

    pub fn recompute_visible(&mut self) {
        let query = self.search_query.to_lowercase();
        let mut rows: Vec<RowKind> = Vec::new();
        let mut has_prev_category = false;

        for cat in &self.categories {
            let cat_apps: Vec<usize> = self.apps.iter().enumerate()
                .filter(|(_, a)| a.category == *cat)
                .filter(|(_, a)| {
                    query.is_empty()
                        || a.name.to_lowercase().contains(&query)
                        || a.description.to_lowercase().contains(&query)
                })
                .map(|(i, _)| i)
                .collect();

            if cat_apps.is_empty() {
                continue;
            }

            if has_prev_category {
                rows.push(RowKind::Spacer);
            }
            has_prev_category = true;

            rows.push(RowKind::CategoryHeader(cat.clone()));
            if !self.collapsed.contains(cat) {
                for idx in cat_apps {
                    rows.push(RowKind::App(idx));
                }
            }
        }

        self.visible_rows = rows;
        if self.cursor >= self.visible_rows.len() {
            self.cursor = self.visible_rows.len().saturating_sub(1);
        }
    }
}

// ── Fetch & parse ───────────────────────────────────────────────────────

fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn cache_path() -> std::path::PathBuf {
    let cwd_config = std::path::PathBuf::from("config");
    if cwd_config.exists() {
        return cwd_config.join(CACHE_FILE);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("../../config");
            if p.exists() {
                return p.join(CACHE_FILE);
            }
        }
    }
    cwd_config.join(CACHE_FILE)
}

fn load_cache() -> Option<Cache> {
    let data = std::fs::read_to_string(cache_path()).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_cache(cache: &Cache) {
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = std::fs::write(cache_path(), json);
    }
}

fn category_order() -> Vec<&'static str> {
    vec![
        "Development Tools",
        "Games",
        "Productivity and Utilities",
        "Music and Media",
        "Networking and Internet",
        "System Administration",
        "Social Media",
        "Embedded",
        "Other",
    ]
}

fn map_category(raw: &str) -> &'static str {
    match raw {
        "Development Tools" => "Development Tools",
        "Games and Entertainment" => "Games",
        "Productivity and Utilities" => "Productivity and Utilities",
        "Music and Media" => "Music and Media",
        "Networking and Internet" => "Networking and Internet",
        "System Administration" => "System Administration",
        "Social Media" => "Social Media",
        "Embedded" => "Embedded",
        "Other" => "Other",
        _ => "Other",
    }
}

fn parse_readme(md: &str) -> Vec<AwesomeApp> {
    let mut apps: Vec<AwesomeApp> = Vec::new();
    let mut in_apps_section = false;
    let mut current_category: Option<String> = None;

    // Category headers we care about (### under ## Apps)
    let cat_headers: HashMap<&str, &str> = [
        ("development tools", "Development Tools"),
        ("games and entertainment", "Games and Entertainment"),
        ("productivity and utilities", "Productivity and Utilities"),
        ("music and media", "Music and Media"),
        ("networking and internet", "Networking and Internet"),
        ("system administration", "System Administration"),
        ("social media", "Social Media"),
        ("embedded", "Embedded"),
        ("other", "Other"),
    ].into_iter().collect();

    for line in md.lines() {
        let trimmed = line.trim();

        // Detect top-level sections
        if trimmed.starts_with("## ") && !trimmed.starts_with("### ") {
            let heading = trimmed.trim_start_matches("## ").trim();
            // Strip emoji prefixes
            let clean = heading.chars().skip_while(|c| !c.is_ascii_alphanumeric()).collect::<String>().trim().to_string();
            in_apps_section = clean.eq_ignore_ascii_case("Apps");
            if !in_apps_section {
                current_category = None;
            }
            continue;
        }

        if !in_apps_section {
            continue;
        }

        // Detect sub-category headings
        if trimmed.starts_with("### ") {
            let heading = trimmed.trim_start_matches("### ").trim();
            let clean: String = heading.chars()
                .skip_while(|c| !c.is_ascii_alphanumeric())
                .collect::<String>()
                .trim()
                .to_string()
                .to_lowercase();

            if let Some(&canonical) = cat_headers.get(clean.as_str()) {
                current_category = Some(canonical.to_string());
            } else {
                current_category = None;
            }
            continue;
        }

        // Parse app entries: "- [Name](url) - Description"
        if let Some(cat) = &current_category {
            if trimmed.starts_with("- [") || trimmed.starts_with("- \n[") {
                if let Some(app) = parse_app_line(trimmed, cat) {
                    apps.push(app);
                }
            }
        }
    }

    apps
}

fn parse_app_line(line: &str, category: &str) -> Option<AwesomeApp> {
    // Format: "- [Name](url) - Description" or "- [Name](url) — Description"
    let rest = line.strip_prefix("- ")?;
    let name_start = rest.find('[')? + 1;
    let name_end = rest.find(']')?;
    let name = rest[name_start..name_end].to_string();

    let url_start = rest.find("](")? + 2;
    let url_end = rest[url_start..].find(')')? + url_start;
    let repo_url = rest[url_start..url_end].to_string();

    // Description comes after ") - " or ") — "
    let after_url = &rest[url_end + 1..];
    let description = after_url
        .trim_start_matches(')')
        .trim()
        .trim_start_matches('-')
        .trim_start_matches('—')
        .trim()
        .to_string();

    let mapped = map_category(category);

    Some(AwesomeApp {
        name,
        repo_url,
        description,
        category: mapped.to_string(),
    })
}

/// Load apps from cache (if fresh) or fetch from GitHub.
pub fn load_or_fetch() -> Result<(Vec<AwesomeApp>, u64), String> {
    if let Some(cache) = load_cache() {
        let age = now_epoch().saturating_sub(cache.fetched_at);
        if age < CACHE_TTL_SECS {
            logging::info(&format!(
                "Awesome Ratatui: loaded {} apps from cache ({}s old)",
                cache.apps.len(), age
            ));
            return Ok((cache.apps, cache.fetched_at));
        }
    }
    fetch_and_cache()
}

/// Force fetch from GitHub regardless of cache.
pub fn fetch_and_cache() -> Result<(Vec<AwesomeApp>, u64), String> {
    logging::info("Awesome Ratatui: fetching README from GitHub...");
    let body = ureq::get(README_URL)
        .call()
        .map_err(|e| format!("HTTP error: {}", e))?
        .into_string()
        .map_err(|e| format!("Read error: {}", e))?;

    let apps = parse_readme(&body);
    let ts = now_epoch();
    logging::info(&format!("Awesome Ratatui: parsed {} apps from README", apps.len()));

    let cache = Cache { fetched_at: ts, apps: apps.clone() };
    save_cache(&cache);

    Ok((apps, ts))
}

/// Build the ordered category list from parsed apps.
pub fn build_categories(apps: &[AwesomeApp]) -> Vec<String> {
    let order = category_order();
    let mut seen: Vec<String> = Vec::new();
    for cat_name in &order {
        if apps.iter().any(|a| a.category == *cat_name) {
            seen.push(cat_name.to_string());
        }
    }
    seen
}

// ── Add / remove from registered apps ───────────────────────────────────

/// Generate a key for a repo URL (last segment of the path, lowercased).
pub fn repo_to_key(repo_url: &str) -> String {
    repo_url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("unknown")
        .to_lowercase()
}

/// Extract author from a GitHub URL.
fn repo_to_author(repo_url: &str) -> String {
    let parts: Vec<&str> = repo_url
        .trim_end_matches('/')
        .rsplitn(3, '/')
        .collect();
    if parts.len() >= 2 { parts[1].to_string() } else { "unknown".to_string() }
}

/// Build a registered-app Value from an AwesomeApp.
pub fn to_registered_value(app: &AwesomeApp) -> Value {
    let key = repo_to_key(&app.repo_url);
    let author = repo_to_author(&app.repo_url);
    serde_json::json!({
        "label": app.name,
        "description": app.description,
        "category": app.category,
        "author": author,
        "version": "unknown",
        "license": "unknown",
        "repository": app.repo_url,
        "crate_name": key,
        "cmd": [key],
        "pre_installed": false,
        "install_methods": ["global", "local"],
        "default_window_mode": "embedded",
        "supports_embedded": true,
        "approved": false
    })
}

/// Add an awesome-ratatui app to registered-apps.json. Returns the key used.
pub fn add_to_registered(app: &AwesomeApp) -> String {
    let key = repo_to_key(&app.repo_url);
    let value = to_registered_value(app);

    let mut registered = crate::tuix::registry::load_registered_apps();
    registered.insert(key.clone(), value);
    crate::tuix::registry::save_config("registered-apps.json", &registered);

    logging::info(&format!(
        "Awesome Ratatui: added '{}' ({}) to registered apps as '{}'",
        app.name, app.category, key
    ));
    key
}

/// Remove an app from registered-apps.json. Only succeeds if not installed.
pub fn remove_from_registered(key: &str, registered: &mut HashMap<String, Value>) -> bool {
    if registered.remove(key).is_some() {
        crate::tuix::registry::save_config("registered-apps.json", registered);
        logging::info(&format!("Awesome Ratatui: removed '{}' from registered apps", key));
        true
    } else {
        false
    }
}


