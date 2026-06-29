/// TUIX — load app/dashboard/settings registries from config/ JSON files.

use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn config_dir() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    // Try relative to CWD first (development), then relative to executable
    let cwd_config = PathBuf::from("config");
    if cwd_config.exists() {
        return cwd_config;
    }

    if let Some(exe) = exe_dir {
        let exe_config = exe.join("../../config");
        if exe_config.exists() {
            return exe_config;
        }
    }

    cwd_config
}

fn load(filename: &str) -> HashMap<String, Value> {
    let path = config_dir().join(filename);
    let content = fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn load_settings() -> HashMap<String, Value> {
    load("settings.json")
}

/// Return dict of {name: {label, module}} from dashboards.json.
pub fn load_dashboards() -> HashMap<String, Value> {
    load("dashboards.json")
}

/// Return dict of {name: {label, type, module/cmd}} from apps.json.
pub fn load_apps() -> HashMap<String, Value> {
    load("apps.json")
}
