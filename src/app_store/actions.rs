/// App Store actions — install, uninstall, detection, and OS integration.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock, atomic::{AtomicBool, Ordering}};
use std::thread;

use serde_json::{json, Value};

use super::state::{InstallLocation, InstallStatus};
use crate::tuix::registry;
use crate::utilities::logging;

// ── Filesystem layout ───────────────────────────────────────────────────

/// Directory that holds `config/` and `downloads/`. Falls back to the CWD.
pub fn project_root() -> PathBuf {
    if Path::new("config").is_dir() || Path::new("downloads").is_dir() {
        return PathBuf::from(".");
    }
    if let Ok(exe) = std::env::current_exe() {
        // target/<profile>/tuix -> walk up looking for the project root
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        for _ in 0..4 {
            let Some(d) = dir else { break };
            if d.join("config").is_dir() || d.join("downloads").is_dir() {
                return d;
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }
    }
    PathBuf::from(".")
}

/// Absolute-ish path of the `downloads/` directory.
pub fn downloads_dir() -> PathBuf {
    project_root().join("downloads")
}

/// Directory where `cargo install` places binaries (`$CARGO_HOME/bin` or `~/.cargo/bin`).
pub fn cargo_bin_dir() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("CARGO_HOME") {
        if !home.is_empty() {
            return Some(PathBuf::from(home).join("bin"));
        }
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .or_else(|| {
            // Windows fallback: HOMEDRIVE + HOMEPATH
            match (std::env::var_os("HOMEDRIVE"), std::env::var_os("HOMEPATH")) {
                (Some(d), Some(p)) => {
                    let mut s = d;
                    s.push(p);
                    Some(s)
                }
                _ => None,
            }
        })?;
    Some(PathBuf::from(home).join(".cargo").join("bin"))
}

/// Executable file names to try for a bare binary stem on the current OS.
fn exe_file_names(stem: &str) -> Vec<String> {
    if cfg!(target_os = "windows") {
        vec![
            format!("{}.exe", stem),
            format!("{}.bat", stem),
            format!("{}.cmd", stem),
            stem.to_string(),
        ]
    } else {
        vec![stem.to_string()]
    }
}

/// Strip an executable extension so `play.exe` and `play` compare equal.
fn bin_stem(file_name: &str) -> String {
    let lower = file_name.to_lowercase();
    for ext in [".exe", ".bat", ".cmd"] {
        if let Some(s) = lower.strip_suffix(ext) {
            return s.to_string();
        }
    }
    lower
}

/// True if `path` is a file we would be willing to execute.
fn is_executable_file(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else { return false };
    if !meta.is_file() {
        return false;
    }
    let name = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    // Build artefacts that live alongside the real binary
    for ext in [".d", ".rlib", ".rmeta", ".pdb", ".so", ".dylib", ".dll", ".a", ".lib", ".exp", ".json", ".txt"] {
        if name.ends_with(ext) {
            return false;
        }
    }
    if name.starts_with('.') {
        return false;
    }
    if cfg!(target_os = "windows") {
        name.ends_with(".exe")
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            meta.permissions().mode() & 0o111 != 0
        }
        #[cfg(not(unix))]
        {
            true
        }
    }
}

// ── `cargo install --list` cache ────────────────────────────────────────

static CARGO_LIST_CACHE: OnceLock<Mutex<Option<HashMap<String, Vec<String>>>>> = OnceLock::new();

fn cargo_list_cell() -> &'static Mutex<Option<HashMap<String, Vec<String>>>> {
    CARGO_LIST_CACHE.get_or_init(|| Mutex::new(None))
}

/// Drop cached views of the cargo install state. Call after any install/uninstall.
pub fn invalidate_install_cache() {
    if let Ok(mut guard) = cargo_list_cell().lock() {
        *guard = None;
    }
}

/// Parse `cargo install --list` into `{package_name: [binary names]}`.
fn parse_cargo_install_list(text: &str) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let indented = line.starts_with(' ') || line.starts_with('\t');
        if !indented {
            // "name vX.Y.Z (source):"
            let head = line.trim().trim_end_matches(':');
            let pkg = head.split_whitespace().next().unwrap_or("").to_string();
            if pkg.is_empty() {
                current = None;
            } else {
                map.entry(pkg.clone()).or_default();
                current = Some(pkg);
            }
        } else if let Some(pkg) = &current {
            let bin = line.trim().to_string();
            if !bin.is_empty() {
                map.entry(pkg.clone()).or_default().push(bin_stem(&bin));
            }
        }
    }
    map
}

/// Cached `cargo install --list` result.
pub fn cargo_install_list() -> HashMap<String, Vec<String>> {
    if let Ok(mut guard) = cargo_list_cell().lock() {
        if let Some(cached) = guard.as_ref() {
            return cached.clone();
        }
        let map = match Command::new("cargo").args(["install", "--list"]).output() {
            Ok(out) if out.status.success() => {
                parse_cargo_install_list(&String::from_utf8_lossy(&out.stdout))
            }
            Ok(_) => HashMap::new(),
            Err(e) => {
                logging::error(&format!("App Store: `cargo install --list` failed: {}", e));
                HashMap::new()
            }
        };
        *guard = Some(map.clone());
        return map;
    }
    HashMap::new()
}

// ── Detection ───────────────────────────────────────────────────────────

/// Every binary name that could plausibly belong to this app, most specific first.
pub fn binary_candidates(app_key: &str, meta: &Value) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |s: &str| {
        let s = bin_stem(s);
        if !s.is_empty() && !out.contains(&s) {
            out.push(s);
        }
    };

    if let Some(b) = meta.get("bin_name").and_then(|v| v.as_str()) {
        push(b);
    }
    // A bare (non-path) first cmd element is a binary name
    if let Some(first) = meta.get("cmd").and_then(|v| v.as_array()).and_then(|a| a.first()).and_then(|v| v.as_str()) {
        if !first.contains('/') && !first.contains('\\') {
            push(first);
        }
    }
    let crate_name = meta.get("crate_name").and_then(|v| v.as_str()).unwrap_or(app_key);
    push(crate_name);
    push(app_key);

    // Binaries cargo actually installed for this package (handles renamed binaries)
    let list = cargo_install_list();
    for pkg in [crate_name, app_key] {
        if let Some(bins) = list.get(pkg) {
            for b in bins {
                push(b);
            }
        }
    }
    out
}

/// Resolve the package name to pass to `cargo uninstall`.
pub fn resolve_package_name(app_key: &str, meta: &Value) -> String {
    let crate_name = meta.get("crate_name").and_then(|v| v.as_str()).unwrap_or(app_key).to_string();
    let list = cargo_install_list();
    if list.contains_key(&crate_name) {
        return crate_name;
    }
    if list.contains_key(app_key) {
        return app_key.to_string();
    }
    // Fall back to whichever installed package owns one of our binaries
    let candidates = binary_candidates(app_key, meta);
    for (pkg, bins) in &list {
        if bins.iter().any(|b| candidates.contains(b)) {
            return pkg.clone();
        }
    }
    crate_name
}

/// Detect a single binary name in PATH or in the cargo bin directory.
pub fn detect_global_install(bin_name: &str) -> Option<String> {
    let stem = bin_stem(bin_name);
    if stem.is_empty() {
        return None;
    }

    // Direct filesystem check first — does not depend on the inherited PATH.
    if let Some(dir) = cargo_bin_dir() {
        for name in exe_file_names(&stem) {
            let p = dir.join(&name);
            if p.is_file() {
                return Some(p.to_string_lossy().to_string());
            }
        }
    }

    let output = if cfg!(target_os = "windows") {
        Command::new("where").arg(&stem).output()
    } else {
        Command::new("which").arg(&stem).output()
    };
    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            // `where` can return several matches, one per line
            text.lines().map(str::trim).find(|l| !l.is_empty()).map(|s| s.to_string())
        }
        _ => None,
    }
}

/// Find the app's binary in PATH / the cargo bin directory.
/// Returns `(binary name, full path)`.
pub fn resolve_global_install(app_key: &str, meta: &Value) -> Option<(String, String)> {
    for cand in binary_candidates(app_key, meta) {
        if let Some(path) = detect_global_install(&cand) {
            return Some((cand, path));
        }
    }
    None
}

/// The `target/release` directory for a locally cloned app.
pub fn local_release_dir(app_key: &str, category: &str) -> PathBuf {
    let subdir = if category == "Dashboard" { "dashboards" } else { "applications" };
    downloads_dir().join(subdir).join(app_key).join("target").join("release")
}

/// Pick the most likely binary inside a `target/release` directory.
/// Prefers a name from `candidates`, otherwise the newest executable file.
pub fn scan_release_binary(dir: &Path, candidates: &[String]) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    for cand in candidates {
        for name in exe_file_names(cand) {
            let p = dir.join(&name);
            if p.is_file() && is_executable_file(&p) {
                return Some(p);
            }
        }
    }

    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if !is_executable_file(&path) {
            continue;
        }
        let mtime = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        if best.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
            best = Some((mtime, path));
        }
    }
    best.map(|(_, p)| p)
}

/// Detect a locally built app under `downloads/`.
pub fn detect_local_install(app_key: &str, category: &str, candidates: &[String]) -> Option<String> {
    let dir = local_release_dir(app_key, category);
    scan_release_binary(&dir, candidates).map(|p| p.to_string_lossy().to_string())
}

/// Get the full install status for a registered app.
pub fn get_install_status(app_key: &str, meta: &Value) -> InstallStatus {
    let category = meta.get("category").and_then(|v| v.as_str()).unwrap_or("Other");
    let candidates = binary_candidates(app_key, meta);

    let global = resolve_global_install(app_key, meta).map(|(_, p)| p);
    let local = detect_local_install(app_key, category, &candidates);

    match (global, local) {
        (Some(g), Some(l)) => InstallStatus::Both(g, l),
        (Some(g), None) => InstallStatus::Global(g),
        (None, Some(l)) => InstallStatus::Local(l),
        (None, None) => InstallStatus::NotInstalled,
    }
}

/// Probe an unregistered app (e.g. one highlighted in the Awesome Ratatui browser)
/// by repo key and display name, checking PATH and both downloads sub-folders.
pub fn probe_install_status(key: &str, display_name: &str) -> InstallStatus {
    let mut names: Vec<String> = Vec::new();
    for n in [key.to_string(), display_name.to_lowercase(), display_name.to_string()] {
        let n = bin_stem(&n);
        if !n.is_empty() && !names.contains(&n) {
            names.push(n);
        }
    }
    // Binaries installed by a package with a matching name, even if renamed
    let list = cargo_install_list();
    for name in names.clone() {
        if let Some(bins) = list.get(&name) {
            for b in bins {
                if !names.contains(b) {
                    names.push(b.clone());
                }
            }
        }
    }

    let global = names.iter().find_map(|n| detect_global_install(n));
    let local = ["applications", "dashboards"].iter().find_map(|sub| {
        let dir = downloads_dir().join(sub).join(key).join("target").join("release");
        scan_release_binary(&dir, &names).map(|p| p.to_string_lossy().to_string())
    });

    match (global, local) {
        (Some(g), Some(l)) => InstallStatus::Both(g, l),
        (Some(g), None) => InstallStatus::Global(g),
        (None, Some(l)) => InstallStatus::Local(l),
        (None, None) => InstallStatus::NotInstalled,
    }
}

/// Refresh install statuses for all registered apps.
pub fn refresh_all_statuses(registered: &HashMap<String, Value>) -> HashMap<String, InstallStatus> {
    invalidate_install_cache();
    registered
        .iter()
        .map(|(key, meta)| (key.clone(), get_install_status(key, meta)))
        .collect()
}

// ── Binary-name resolution after an install ─────────────────────────────

/// File stems currently present in the cargo bin directory.
pub fn snapshot_cargo_bin() -> HashSet<String> {
    let Some(dir) = cargo_bin_dir() else { return HashSet::new() };
    let Ok(entries) = std::fs::read_dir(&dir) else { return HashSet::new() };
    entries
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| bin_stem(&e.file_name().to_string_lossy()))
        .collect()
}

/// Parse cargo's `Installed package \`foo v1.0\` (executable \`play\`)` line.
fn parse_installed_executables(lines: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for line in lines {
        let lower = line.to_lowercase();
        if !lower.contains("executable") {
            continue;
        }
        let Some(start) = lower.find("executable") else { continue };
        for chunk in line[start..].split('`').skip(1).step_by(2) {
            let name = bin_stem(chunk.trim());
            if !name.is_empty() && !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out
}

/// Work out which binary a global/git install produced.
/// Tries, in order: newly appeared files in the cargo bin dir, cargo's own
/// "executable" output line, then `cargo install --list`.
pub fn resolve_installed_binary(
    app_key: &str,
    meta: &Value,
    before: &HashSet<String>,
    output: &[String],
) -> Option<String> {
    invalidate_install_cache();

    let after = snapshot_cargo_bin();
    let mut added: Vec<String> = after.difference(before).cloned().collect();
    if added.len() > 1 {
        added.sort();
    }
    if let Some(name) = added.first() {
        return Some(name.clone());
    }

    if let Some(name) = parse_installed_executables(output).into_iter().next() {
        return Some(name);
    }

    let crate_name = meta.get("crate_name").and_then(|v| v.as_str()).unwrap_or(app_key);
    let list = cargo_install_list();
    for pkg in [crate_name, app_key] {
        if let Some(b) = list.get(pkg).and_then(|b| b.first()) {
            return Some(b.clone());
        }
    }

    // Last resort: any candidate that now exists on disk
    resolve_global_install(app_key, meta).map(|(name, _)| name)
}

/// Work out which binary a local (`downloads/`) build produced.
pub fn resolve_built_binary(app_key: &str, meta: &Value) -> Option<String> {
    let category = meta.get("category").and_then(|v| v.as_str()).unwrap_or("Other");
    let candidates = binary_candidates(app_key, meta);
    let dir = local_release_dir(app_key, category);
    scan_release_binary(&dir, &candidates)
        .and_then(|p| p.file_name().map(|n| bin_stem(&n.to_string_lossy())))
}

/// Persist the discovered binary name into registered-apps.json so future
/// detection and launches use the right command.
pub fn record_binary_name(app_key: &str, bin_name: &str) {
    let mut registered = registry::load_registered_apps();
    let Some(entry) = registered.get_mut(app_key) else { return };
    let Some(obj) = entry.as_object_mut() else { return };

    let unchanged = obj.get("bin_name").and_then(|v| v.as_str()) == Some(bin_name);
    if unchanged {
        return;
    }
    obj.insert("bin_name".into(), Value::String(bin_name.to_string()));
    obj.insert("cmd".into(), Value::Array(vec![Value::String(bin_name.to_string())]));
    registry::save_config("registered-apps.json", &registered);
    logging::info(&format!(
        "App Store: resolved binary for {} -> {}",
        app_key, bin_name
    ));
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
    let target_dir = downloads_dir().join(subdir).join(app_key);
    let target_dir = target_dir.to_string_lossy().to_string();

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

/// Build the `cmd` array for an installed app, using the binary that was
/// actually produced rather than assuming it matches the app name.
fn resolve_cmd(app_key: &str, meta: &Value, location: &InstallLocation) -> Vec<Value> {
    let category = meta.get("category").and_then(|v| v.as_str()).unwrap_or("Other");
    match location {
        InstallLocation::Local => {
            let candidates = binary_candidates(app_key, meta);
            let dir = local_release_dir(app_key, category);
            match scan_release_binary(&dir, &candidates) {
                Some(p) => vec![Value::String(p.to_string_lossy().to_string())],
                None => Vec::new(),
            }
        }
        InstallLocation::Global | InstallLocation::Git => {
            match resolve_global_install(app_key, meta) {
                Some((bin, _)) => vec![Value::String(bin)],
                None => meta
                    .get("cmd")
                    .and_then(|c| c.as_array())
                    .cloned()
                    .unwrap_or_default(),
            }
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
        InstallLocation::Global | InstallLocation::Git => "global",
        InstallLocation::Local => "local",
    };

    let cmd = resolve_cmd(app_key, meta, location);

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

/// Sync apps.json and dashboards.json with what's actually installed on disk/PATH.
/// Adds entries for apps found installed but not in config, removes entries for apps
/// no longer found, and repairs `cmd` entries whose binary name has changed.
/// Returns true if any changes were made.
pub fn sync_installed_apps(registered: &HashMap<String, Value>) -> bool {
    invalidate_install_cache();

    let mut changed = false;
    let mut apps_config = registry::load_apps();
    let mut dash_config = registry::load_dashboards();
    let mut registered_updates: Vec<(String, String)> = Vec::new();

    for (key, meta) in registered {
        let category = meta.get("category").and_then(|v| v.as_str()).unwrap_or("Other");
        let config = if category == "Dashboard" { &mut dash_config } else { &mut apps_config };
        let status = get_install_status(key, meta);

        if matches!(status, InstallStatus::NotInstalled) {
            if config.remove(key).is_some() {
                logging::info(&format!("App Store sync: removed {} (no longer installed)", key));
                changed = true;
            }
            continue;
        }

        let location = match &status {
            InstallStatus::Local(_) => InstallLocation::Local,
            _ => InstallLocation::Global,
        };

        // Remember the real binary name so detection and launching stay correct
        let discovered = match &status {
            InstallStatus::Local(_) => resolve_built_binary(key, meta),
            _ => resolve_global_install(key, meta).map(|(bin, _)| bin),
        };
        if let Some(bin) = &discovered {
            if meta.get("bin_name").and_then(|v| v.as_str()) != Some(bin.as_str()) {
                registered_updates.push((key.clone(), bin.clone()));
            }
        }

        let cmd = resolve_cmd(key, meta, &location);
        let source = match &location {
            InstallLocation::Local => "local",
            _ => "global",
        };
        let label = meta.get("label").and_then(|v| v.as_str()).unwrap_or(key);
        let version = meta.get("version").and_then(|v| v.as_str()).unwrap_or("unknown");
        let repo = meta.get("repository").and_then(|v| v.as_str()).unwrap_or("");
        let entry = json!({
            "label": label,
            "type": "installed",
            "source": source,
            "repository": repo,
            "version": version,
            "cmd": cmd,
        });

        match config.get(key) {
            Some(existing) if existing == &entry => {}
            Some(_) => {
                config.insert(key.clone(), entry);
                logging::info(&format!("App Store sync: updated {} (install details changed)", key));
                changed = true;
            }
            None => {
                config.insert(key.clone(), entry);
                logging::info(&format!("App Store sync: added {} (found installed)", key));
                changed = true;
            }
        }
    }

    if !registered_updates.is_empty() {
        for (key, bin) in registered_updates {
            record_binary_name(&key, &bin);
        }
    }

    if changed {
        registry::save_config("apps.json", &apps_config);
        registry::save_config("dashboards.json", &dash_config);
        logging::info("App Store sync: configs updated");
    }

    changed
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

/// Resolve how to launch an installed app or dashboard.
///
/// Uses the binary that is actually present rather than assuming it is named
/// after the app, and honours the saved PATH/Downloads run-source preference.
/// Falls back to the stored `cmd` when nothing can be found on disk.
pub fn resolve_launch_cmd(
    app_key: &str,
    category: &str,
    registered_meta: Option<&Value>,
    config_meta: Option<&Value>,
) -> Vec<String> {
    let meta = registered_meta
        .or(config_meta)
        .cloned()
        .unwrap_or(Value::Null);
    let candidates = binary_candidates(app_key, &meta);

    let global = resolve_global_install(app_key, &meta);
    let local = detect_local_install(app_key, category, &candidates);

    match (global, local) {
        (Some((bin, _)), Some(local_path)) => {
            if get_run_source(app_key) == "local" {
                logging::info(&format!("Running {} from Downloads source", app_key));
                vec![local_path]
            } else {
                logging::info(&format!("Running {} from PATH source", app_key));
                vec![bin]
            }
        }
        (Some((bin, _)), None) => vec![bin],
        (None, Some(local_path)) => vec![local_path],
        (None, None) => config_meta
            .or(registered_meta)
            .and_then(|m| m.get("cmd"))
            .and_then(|c| c.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default(),
    }
}

/// Get the preferred run source for an app ("global" or "local"). Defaults to "global".
pub fn get_run_source(app_key: &str) -> String {
    let settings = crate::settings::persistence::load();
    let path = format!("app_store.run_sources.{}", app_key);
    crate::settings::persistence::get_str(&settings, &path, "global")
}

/// Set the preferred run source for an app.
pub fn set_run_source(app_key: &str, source: &str) {
    let mut settings = crate::settings::persistence::load();
    let path = format!("app_store.run_sources.{}", app_key);
    crate::settings::persistence::set(&mut settings, &path, serde_json::Value::String(source.to_string()));
    crate::settings::persistence::save(&settings);
    logging::info(&format!("App Store: set run source for {} to {}", app_key, source));
}

/// Get the window mode for an app ("embedded" or "fullscreen"). Falls back to registered default.
pub fn get_window_mode(app_key: &str, registered_meta: Option<&serde_json::Value>) -> String {
    let settings = crate::settings::persistence::load();
    let path = format!("app_store.window_modes.{}", app_key);
    let saved = crate::settings::persistence::get_str(&settings, &path, "");
    if !saved.is_empty() {
        return saved;
    }
    // Fall back to registered-apps.json default
    registered_meta
        .and_then(|m| m.get("default_window_mode"))
        .and_then(|v| v.as_str())
        .unwrap_or("embedded")
        .to_string()
}

/// Set the window mode for an app.
pub fn set_window_mode(app_key: &str, mode: &str) {
    let mut settings = crate::settings::persistence::load();
    let path = format!("app_store.window_modes.{}", app_key);
    crate::settings::persistence::set(&mut settings, &path, serde_json::Value::String(mode.to_string()));
    crate::settings::persistence::save(&settings);
    logging::info(&format!("App Store: set window mode for {} to {}", app_key, mode));
}

/// Check if an app supports running embedded in TUIX.
pub fn supports_embedded(registered_meta: Option<&serde_json::Value>) -> bool {
    registered_meta
        .and_then(|m| m.get("supports_embedded"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
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
        invalidate_install_cache();
        done.store(true, Ordering::Relaxed);
    });
}

/// Spawn `cargo install --git <repo>` in a background thread.
pub fn spawn_install_git(repo_url: &str, output: Arc<Mutex<Vec<String>>>, done: Arc<AtomicBool>, success: Arc<AtomicBool>) {
    let repo_url = repo_url.to_string();
    thread::spawn(move || {
        let ok = run_streaming("cargo", &["install", "--git", &repo_url], &output);
        if ok {
            push_output(&output, format!("✓ Successfully installed from {}", repo_url));
            success.store(true, Ordering::Relaxed);
        } else {
            push_output(&output, format!("✗ Failed to install from {}", repo_url));
            logging::error(&format!("App Store: cargo install --git failed for {}", repo_url));
        }
        invalidate_install_cache();
        done.store(true, Ordering::Relaxed);
    });
}

/// Install options to offer for an app. `cargo install --git` is available for
/// anything with a repository URL, since it does not require a crates.io release.
pub fn install_methods_for(meta: &Value) -> Vec<InstallLocation> {
    let declared: Vec<&str> = meta
        .get("install_methods")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_else(|| vec!["global", "local"]);
    let has_repo = meta
        .get("repository")
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    let mut out = Vec::new();
    if declared.contains(&"global") {
        out.push(InstallLocation::Global);
    }
    if has_repo {
        out.push(InstallLocation::Git);
    }
    if declared.contains(&"local") && has_repo {
        out.push(InstallLocation::Local);
    }
    if out.is_empty() {
        out.push(InstallLocation::Global);
    }
    out
}

/// Kick off an install in the background for the chosen location.
pub fn spawn_install(
    location: InstallLocation,
    app_key: &str,
    meta: &Value,
    output: Arc<Mutex<Vec<String>>>,
    done: Arc<AtomicBool>,
    success: Arc<AtomicBool>,
) {
    let repo = meta.get("repository").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let category = meta.get("category").and_then(|v| v.as_str()).unwrap_or("Other").to_string();
    match location {
        InstallLocation::Global => {
            let crate_name = meta
                .get("crate_name")
                .and_then(|v| v.as_str())
                .unwrap_or(app_key)
                .to_string();
            spawn_install_global(&crate_name, output, done, success);
        }
        InstallLocation::Git => spawn_install_git(&repo, output, done, success),
        InstallLocation::Local => spawn_install_local(app_key, &repo, &category, output, done, success),
    }
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
        let target_dir = downloads_dir().join(subdir).join(&app_key).to_string_lossy().to_string();
        let _ = std::fs::create_dir_all(downloads_dir().join(subdir));
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
        invalidate_install_cache();
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
        invalidate_install_cache();
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
        let target_dir = downloads_dir().join(subdir).join(&app_key).to_string_lossy().to_string();
        push_output(&output, format!("Removing directory: {}", target_dir));
        match std::fs::remove_dir_all(&target_dir) {
            Ok(_) => push_output(&output, format!("✓ Successfully removed {}", target_dir)),
            Err(e) => push_output(&output, format!("✗ Error removing {}: {}", target_dir, e)),
        }
        done.store(true, Ordering::Relaxed);
    });
}
