/// App Store actions — install, uninstall, detection, and OS integration.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;

use serde_json::{json, Value};

use super::state::{InstallLocation, InstallStatus};
use crate::tuix::registry;
use crate::utilities::logging;

/// Detect if a binary is installed globally (in PATH).
pub fn detect_global_install(crate_name: &str) -> Option<String> {
    let cmd = if cfg!(target_os = "windows") {
        Command::new("where").arg(crate_name).output()
    } else {
        Command::new("which").arg(crate_name).output()
    };
    match cmd {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if path.is_empty() { None } else { Some(path) }
        }
        _ => None,
    }
}

/// Detect if an app is built locally in the downloads directory.
pub fn detect_local_install(app_name: &str, category: &str) -> Option<String> {
    let subdir = if category == "Dashboard" { "dashboards" } else { "applications" };
    let bin_name = if cfg!(target_os = "windows") {
        format!("{}.exe", app_name)
    } else {
        app_name.to_string()
    };
    let path = PathBuf::from(format!("downloads/{}/{}/target/release/{}", subdir, app_name, bin_name));
    if path.exists() {
        Some(path.to_string_lossy().to_string())
    } else {
        None
    }
}

/// Get the full install status for a registered app.
pub fn get_install_status(app_key: &str, meta: &Value) -> InstallStatus {
    let crate_name = meta.get("crate_name").and_then(|v| v.as_str()).unwrap_or(app_key);
    let category = meta.get("category").and_then(|v| v.as_str()).unwrap_or("Other");

    let global = detect_global_install(crate_name);
    let local = detect_local_install(app_key, category);

    match (global, local) {
        (Some(g), Some(l)) => InstallStatus::Both(g, l),
        (Some(g), None) => InstallStatus::Global(g),
        (None, Some(l)) => InstallStatus::Local(l),
        (None, None) => InstallStatus::NotInstalled,
    }
}

/// Refresh install statuses for all registered apps.
pub fn refresh_all_statuses(registered: &HashMap<String, Value>) -> HashMap<String, InstallStatus> {
    registered
        .iter()
        .map(|(key, meta)| (key.clone(), get_install_status(key, meta)))
        .collect()
}

/// Install an app globally via `cargo install`.
#[allow(dead_code)]
pub fn install_global(crate_name: &str, output: &mut Vec<String>) {
    let cmd_str = format!("cargo install {}", crate_name);
    output.push(format!("$ {}", cmd_str));
    logging::info(&format!("App Store: installing globally: {}", crate_name));

    match Command::new("cargo")
        .args(["install", crate_name])
        .output()
    {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);
            for line in stdout.lines() {
                output.push(line.to_string());
            }
            for line in stderr.lines() {
                output.push(line.to_string());
            }
            if result.status.success() {
                output.push(format!("Successfully installed {}", crate_name));
                logging::info(&format!("App Store: {} installed globally", crate_name));
            } else {
                output.push(format!("Failed to install {} (exit code: {:?})", crate_name, result.status.code()));
                logging::error(&format!("App Store: failed to install {} globally", crate_name));
            }
        }
        Err(e) => {
            output.push(format!("Error: {}", e));
            logging::error(&format!("App Store: cargo install error: {}", e));
        }
    }
}

/// Install an app locally via `git clone` + `cargo build --release`.
#[allow(dead_code)]
pub fn install_local(app_key: &str, repo_url: &str, category: &str, output: &mut Vec<String>) {
    let subdir = if category == "Dashboard" { "dashboards" } else { "applications" };
    let target_dir = format!("downloads/{}/{}", subdir, app_key);

    // Create parent directory
    let _ = std::fs::create_dir_all(format!("downloads/{}", subdir));

    // Clone
    let clone_cmd = format!("git clone {} {}", repo_url, target_dir);
    output.push(format!("$ {}", clone_cmd));
    logging::info(&format!("App Store: cloning {} to {}", repo_url, target_dir));

    if std::path::Path::new(&target_dir).exists() {
        output.push(format!("Directory {} already exists, pulling instead...", target_dir));
        match Command::new("git")
            .args(["-C", &target_dir, "pull"])
            .output()
        {
            Ok(result) => {
                for line in String::from_utf8_lossy(&result.stdout).lines() {
                    output.push(line.to_string());
                }
                for line in String::from_utf8_lossy(&result.stderr).lines() {
                    output.push(line.to_string());
                }
            }
            Err(e) => {
                output.push(format!("Git pull error: {}", e));
                logging::error(&format!("App Store: git pull error: {}", e));
                return;
            }
        }
    } else {
        match Command::new("git")
            .args(["clone", repo_url, &target_dir])
            .output()
        {
            Ok(result) => {
                for line in String::from_utf8_lossy(&result.stdout).lines() {
                    output.push(line.to_string());
                }
                for line in String::from_utf8_lossy(&result.stderr).lines() {
                    output.push(line.to_string());
                }
                if !result.status.success() {
                    output.push("Git clone failed.".into());
                    logging::error(&format!("App Store: git clone failed for {}", app_key));
                    return;
                }
            }
            Err(e) => {
                output.push(format!("Error: {}", e));
                logging::error(&format!("App Store: git clone error: {}", e));
                return;
            }
        }
    }

    // Remove .git and .github directories to save space
    let git_dir = format!("{}/.git", target_dir);
    let github_dir = format!("{}/.github", target_dir);
    if std::path::Path::new(&git_dir).exists() {
        let _ = std::fs::remove_dir_all(&git_dir);
        output.push("Removed .git directory".into());
    }
    if std::path::Path::new(&github_dir).exists() {
        let _ = std::fs::remove_dir_all(&github_dir);
        output.push("Removed .github directory".into());
    }

    // Build
    let manifest = format!("{}/Cargo.toml", target_dir);
    let build_cmd = format!("cargo build --release --manifest-path {}", manifest);
    output.push(format!("$ {}", build_cmd));
    logging::info(&format!("App Store: building {}", app_key));

    match Command::new("cargo")
        .args(["build", "--release", "--manifest-path", &manifest])
        .output()
    {
        Ok(result) => {
            for line in String::from_utf8_lossy(&result.stdout).lines() {
                output.push(line.to_string());
            }
            for line in String::from_utf8_lossy(&result.stderr).lines() {
                output.push(line.to_string());
            }
            if result.status.success() {
                output.push(format!("Successfully built {}", app_key));
                logging::info(&format!("App Store: {} built successfully", app_key));
            } else {
                output.push(format!("Build failed for {}", app_key));
                logging::error(&format!("App Store: build failed for {}", app_key));
            }
        }
        Err(e) => {
            output.push(format!("Error: {}", e));
            logging::error(&format!("App Store: cargo build error: {}", e));
        }
    }
}

/// Uninstall an app globally via `cargo uninstall`.
pub fn uninstall_global(crate_name: &str, output: &mut Vec<String>) {
    let cmd_str = format!("cargo uninstall {}", crate_name);
    output.push(format!("$ {}", cmd_str));
    logging::info(&format!("App Store: uninstalling globally: {}", crate_name));

    match Command::new("cargo")
        .args(["uninstall", crate_name])
        .output()
    {
        Ok(result) => {
            for line in String::from_utf8_lossy(&result.stdout).lines() {
                output.push(line.to_string());
            }
            for line in String::from_utf8_lossy(&result.stderr).lines() {
                output.push(line.to_string());
            }
            if result.status.success() {
                output.push(format!("Successfully uninstalled {}", crate_name));
                logging::info(&format!("App Store: {} uninstalled globally", crate_name));
            } else {
                output.push(format!("Failed to uninstall {}", crate_name));
                logging::error(&format!("App Store: failed to uninstall {}", crate_name));
            }
        }
        Err(e) => {
            output.push(format!("Error: {}", e));
            logging::error(&format!("App Store: cargo uninstall error: {}", e));
        }
    }
}

/// Uninstall an app locally by removing the downloads directory.
pub fn uninstall_local(app_key: &str, category: &str, output: &mut Vec<String>) {
    let subdir = if category == "Dashboard" { "dashboards" } else { "applications" };
    let target_dir = format!("downloads/{}/{}", subdir, app_key);

    output.push(format!("Removing directory: {}", target_dir));
    logging::info(&format!("App Store: removing local install: {}", target_dir));

    match std::fs::remove_dir_all(&target_dir) {
        Ok(_) => {
            output.push(format!("Successfully removed {}", target_dir));
            logging::info(&format!("App Store: {} removed locally", app_key));
        }
        Err(e) => {
            output.push(format!("Error removing {}: {}", target_dir, e));
            logging::error(&format!("App Store: failed to remove {}: {}", target_dir, e));
        }
    }
}

/// Add an installed app entry to apps.json or dashboards.json.
pub fn add_to_config(
    app_key: &str,
    meta: &Value,
    location: &InstallLocation,
) {
    let category = meta.get("category").and_then(|v| v.as_str()).unwrap_or("Other");
    let config_file = if category == "Dashboard" { "dashboards.json" } else { "apps.json" };
    let mut config = registry::load_apps();
    if category == "Dashboard" {
        config = registry::load_dashboards();
    }

    let label = meta.get("label").and_then(|v| v.as_str()).unwrap_or(app_key);
    let version = meta.get("version").and_then(|v| v.as_str()).unwrap_or("unknown");
    let repo = meta.get("repository").and_then(|v| v.as_str()).unwrap_or("");

    let source = match location {
        InstallLocation::Global => "global",
        InstallLocation::Local => "local",
    };

    // For local installs, point cmd to the built binary
    let cmd: Vec<Value> = match location {
        InstallLocation::Local => {
            let subdir = if category == "Dashboard" { "dashboards" } else { "applications" };
            let bin_name = meta.get("crate_name").and_then(|v| v.as_str()).unwrap_or(app_key);
            let bin_path = format!("downloads/{}/{}/target/release/{}", subdir, app_key, bin_name);
            vec![Value::String(bin_path)]
        }
        InstallLocation::Global => {
            meta.get("cmd")
                .and_then(|c| c.as_array())
                .cloned()
                .unwrap_or_default()
        }
    };

    let entry = json!({
        "label": label,
        "type": "installed",
        "source": source,
        "repository": repo,
        "version": version,
        "cmd": cmd,
    });

    config.insert(app_key.to_string(), entry);
    registry::save_config(config_file, &config);
    logging::info(&format!("App Store: added {} to {}", app_key, config_file));
}

/// Remove an app entry from apps.json or dashboards.json.
pub fn remove_from_config(app_key: &str, category: &str) {
    let config_file = if category == "Dashboard" { "dashboards.json" } else { "apps.json" };
    let mut config = if category == "Dashboard" {
        registry::load_dashboards()
    } else {
        registry::load_apps()
    };

    config.remove(app_key);
    registry::save_config(config_file, &config);
    logging::info(&format!("App Store: removed {} from {}", app_key, config_file));
}

/// Open a URL in the system browser.
pub fn open_url(url: &str) -> bool {
    logging::info(&format!("App Store: opening URL: {}", url));
    let result = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", "", url]).spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()
    } else {
        Command::new("xdg-open").arg(url).spawn()
    };
    result.is_ok()
}

/// Open a directory in the OS file manager.
pub fn open_in_os_explorer(path: &str) -> bool {
    logging::info(&format!("App Store: opening in OS file manager: {}", path));
    let result = if cfg!(target_os = "windows") {
        Command::new("explorer").arg(path).spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(path).spawn()
    } else {
        Command::new("xdg-open").arg(path).spawn()
    };
    result.is_ok()
}

/// Get the parent directory of a binary path for file explorer navigation.
pub fn install_dir_from_path(path: &str) -> String {
    PathBuf::from(path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

// --- Thread-spawning variants for non-blocking UI ---

pub fn push_output(buf: &Arc<Mutex<Vec<String>>>, line: String) {
    if let Ok(mut v) = buf.lock() {
        v.push(line);
    }
}

/// Run a command, streaming stdout and stderr line-by-line into the shared buffer.
fn run_streaming(cmd: &str, args: &[&str], output: &Arc<Mutex<Vec<String>>>) -> bool {
    use std::io::BufRead;
    use std::process::Stdio;

    push_output(output, format!("$ {} {}", cmd, args.join(" ")));

    let mut child = match Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            push_output(output, format!("Error: {}", e));
            return false;
        }
    };

    // Read stderr in a thread (cargo writes progress here)
    let stderr = child.stderr.take();
    let out_clone = output.clone();
    let stderr_handle = thread::spawn(move || {
        if let Some(stderr) = stderr {
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(l) = line {
                    push_output(&out_clone, l);
                }
            }
        }
    });

    // Read stdout in main thread
    if let Some(stdout) = child.stdout.take() {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line {
                push_output(output, l);
            }
        }
    }

    let _ = stderr_handle.join();
    match child.wait() {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

/// Spawn install_global in a background thread.
pub fn spawn_install_global(crate_name: &str, output: Arc<Mutex<Vec<String>>>, done: Arc<AtomicBool>, success: Arc<AtomicBool>) {
    let crate_name = crate_name.to_string();
    thread::spawn(move || {
        let ok = run_streaming("cargo", &["install", &crate_name], &output);
        if ok {
            push_output(&output, format!("✓ Successfully installed {}", crate_name));
            success.store(true, Ordering::Relaxed);
        } else {
            push_output(&output, format!("✗ Failed to install {}", crate_name));
            logging::error(&format!("App Store: cargo install failed for {}", crate_name));
        }
        done.store(true, Ordering::Relaxed);
    });
}

/// Spawn install_local in a background thread.
pub fn spawn_install_local(
    app_key: &str,
    repo_url: &str,
    category: &str,
    output: Arc<Mutex<Vec<String>>>,
    done: Arc<AtomicBool>,
    success: Arc<AtomicBool>,
) {
    let app_key = app_key.to_string();
    let repo_url = repo_url.to_string();
    let category = category.to_string();
    thread::spawn(move || {
        let subdir = if category == "Dashboard" { "dashboards" } else { "applications" };
        let target_dir = format!("downloads/{}/{}", subdir, app_key);
        let _ = std::fs::create_dir_all(format!("downloads/{}", subdir));
        let freshly_cloned = !std::path::Path::new(&target_dir).exists();

        if !freshly_cloned {
            push_output(&output, format!("Directory {} already exists, pulling...", target_dir));
            let ok = run_streaming("git", &["-C", &target_dir, "pull"], &output);
            if !ok {
                push_output(&output, "✗ Git pull failed.".into());
                logging::error(&format!("App Store: git pull failed for {}", app_key));
                done.store(true, Ordering::Relaxed);
                return;
            }
        } else {
            let ok = run_streaming("git", &["clone", &repo_url, &target_dir], &output);
            if !ok {
                push_output(&output, "✗ Git clone failed — cleaning up.".into());
                logging::error(&format!("App Store: git clone failed for {}", app_key));
                let _ = std::fs::remove_dir_all(&target_dir);
                done.store(true, Ordering::Relaxed);
                return;
            }
        }

        // Remove .git and .github
        let git_dir = format!("{}/.git", target_dir);
        let github_dir = format!("{}/.github", target_dir);
        if std::path::Path::new(&git_dir).exists() {
            let _ = std::fs::remove_dir_all(&git_dir);
            push_output(&output, "Removed .git directory".into());
        }
        if std::path::Path::new(&github_dir).exists() {
            let _ = std::fs::remove_dir_all(&github_dir);
            push_output(&output, "Removed .github directory".into());
        }

        // Build with live streaming
        let manifest = format!("{}/Cargo.toml", target_dir);
        let ok = run_streaming("cargo", &["build", "--release", "--manifest-path", &manifest], &output);
        if ok {
            push_output(&output, format!("✓ Successfully built {}", app_key));
            success.store(true, Ordering::Relaxed);
        } else {
            push_output(&output, format!("✗ Build failed for {} — cleaning up.", app_key));
            logging::error(&format!("App Store: cargo build failed for {}", app_key));
            if freshly_cloned {
                let _ = std::fs::remove_dir_all(&target_dir);
                push_output(&output, "Removed partial install directory.".into());
            }
        }
        done.store(true, Ordering::Relaxed);
    });
}

/// Spawn uninstall_global in a background thread.
#[allow(dead_code)]
pub fn spawn_uninstall_global(crate_name: &str, output: Arc<Mutex<Vec<String>>>, done: Arc<AtomicBool>) {
    let crate_name = crate_name.to_string();
    thread::spawn(move || {
        let ok = run_streaming("cargo", &["uninstall", &crate_name], &output);
        if ok {
            push_output(&output, format!("✓ Successfully uninstalled {}", crate_name));
        } else {
            push_output(&output, format!("✗ Failed to uninstall {}", crate_name));
        }
        done.store(true, Ordering::Relaxed);
    });
}

/// Spawn uninstall_local in a background thread.
#[allow(dead_code)]
pub fn spawn_uninstall_local(app_key: &str, category: &str, output: Arc<Mutex<Vec<String>>>, done: Arc<AtomicBool>) {
    let app_key = app_key.to_string();
    let category = category.to_string();
    thread::spawn(move || {
        let subdir = if category == "Dashboard" { "dashboards" } else { "applications" };
        let target_dir = format!("downloads/{}/{}", subdir, app_key);
        push_output(&output, format!("Removing directory: {}", target_dir));
        match std::fs::remove_dir_all(&target_dir) {
            Ok(_) => push_output(&output, format!("✓ Successfully removed {}", target_dir)),
            Err(e) => push_output(&output, format!("✗ Error removing {}: {}", target_dir, e)),
        }
        done.store(true, Ordering::Relaxed);
    });
}
