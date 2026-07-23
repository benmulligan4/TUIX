/// Settings persistence — read/write config/settings.json.

use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn settings_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let cwd = std::env::current_dir().unwrap_or_default();
    let base = exe_dir.unwrap_or(cwd);

    // Look for config/ relative to CWD first (dev), then next to the exe
    let cwd_path = std::env::current_dir()
        .unwrap_or_default()
        .join("config")
        .join("settings.json");
    if cwd_path.exists() {
        return cwd_path;
    }
    base.join("config").join("settings.json")
}

/// Load the full settings.json as a serde_json::Value.
pub fn load() -> Value {
    let path = settings_path();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or(Value::Object(Default::default())),
        Err(_) => Value::Object(Default::default()),
    }
}

/// Save the full settings Value back to settings.json.
pub fn save(settings: &Value) {
    let path = settings_path();
    if let Ok(content) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(&path, content);
    }
}

/// Get a nested value by dot-separated path (e.g. "appearance.accent_color").
pub fn get(settings: &Value, path: &str) -> Option<Value> {
    let mut current = settings;
    for key in path.split('.') {
        current = current.get(key)?;
    }
    Some(current.clone())
}

/// Set a nested value by dot-separated path. Creates intermediate objects if needed.
pub fn set(settings: &mut Value, path: &str, value: Value) {
    let keys: Vec<&str> = path.split('.').collect();
    let mut current = settings;
    for &key in &keys[..keys.len() - 1] {
        if !current.get(key).map_or(false, |v| v.is_object()) {
            current[key] = Value::Object(Default::default());
        }
        current = current.get_mut(key).unwrap();
    }
    if let Some(last_key) = keys.last() {
        current[*last_key] = value;
    }
}

/// Get a string setting, with a default fallback.
pub fn get_str<'a>(settings: &'a Value, path: &str, default: &'a str) -> String {
    get(settings, path)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| default.to_string())
}

/// Get a bool setting, with a default fallback.
pub fn get_bool(settings: &Value, path: &str, default: bool) -> bool {
    get(settings, path)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

/// Convenience: load settings, then look up a list of dashboard names from registry.
pub fn load_dashboard_names() -> Vec<String> {
    let path = std::env::current_dir()
        .unwrap_or_default()
        .join("config")
        .join("dashboards.json");
    match fs::read_to_string(&path) {
        Ok(content) => {
            let map: HashMap<String, Value> =
                serde_json::from_str(&content).unwrap_or_default();
            map.keys().cloned().collect()
        }
        Err(_) => vec![],
    }
}

/// Convenience: load app names from registry.
pub fn load_app_names() -> Vec<String> {
    let path = std::env::current_dir()
        .unwrap_or_default()
        .join("config")
        .join("apps.json");
    match fs::read_to_string(&path) {
        Ok(content) => {
            let map: HashMap<String, Value> =
                serde_json::from_str(&content).unwrap_or_default();
            map.keys().cloned().collect()
        }
        Err(_) => vec![],
    }
}
