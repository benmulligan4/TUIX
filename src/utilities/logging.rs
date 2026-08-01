/// TUIX Logging
///
/// Writes to `tuix.log` at the project root. Thread-safe via a global mutex.
/// Cross-platform (Windows + Linux/Raspberry Pi).
///
/// Severity levels: INFO, WARN, ERROR
/// Format: [YYYY-MM-DD HH:MM:SS] [LEVEL] message

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::Local;
use lazy_static::lazy_static;

/// Severity levels for log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Level {
    Info,
    Warn,
    Error,
    Script,
    Settings,
}

impl Level {
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
            Level::Script => "SCRIPT",
            Level::Settings => "SETTINGS",
        }
    }
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

lazy_static! {
    static ref LOG_FILE: Mutex<Option<File>> = Mutex::new(None);
    static ref LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
}

/// Initialize the logger. Creates the log file if it doesn't exist.
/// Call this once at startup.
pub fn init() {
    let path = log_file_path();
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path);

    match file {
        Ok(f) => {
            *LOG_PATH.lock().unwrap() = Some(path);
            *LOG_FILE.lock().unwrap() = Some(f);
        }
        Err(e) => {
            eprintln!("Failed to open log file: {}", e);
        }
    }
}

/// Returns the path to the log file (project root / tuix.log).
fn log_file_path() -> PathBuf {
    // Use CWD (project root when running via cargo run)
    PathBuf::from("tuix.log")
}

/// Get the log file path for reading (e.g. by the log viewer).
pub fn get_log_path() -> PathBuf {
    if let Ok(guard) = LOG_PATH.lock() {
        if let Some(ref p) = *guard {
            return p.clone();
        }
    }
    log_file_path()
}

/// Write a log entry.
pub fn log(level: Level, message: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let entry = format!("[{}] [{}] {}\n", timestamp, level.as_str(), message);

    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut file) = *guard {
            let _ = file.write_all(entry.as_bytes());
            let _ = file.flush();
        }
    }
}

/// Log an INFO message.
pub fn info(message: &str) {
    log(Level::Info, message);
}

/// Log a WARN message.
#[allow(dead_code)]
pub fn warn(message: &str) {
    log(Level::Warn, message);
}

/// Log an ERROR message.
#[allow(dead_code)]
pub fn error(message: &str) {
    log(Level::Error, message);
}

/// Log a SETTINGS message.
#[allow(dead_code)]
pub fn settings(message: &str) {
    log(Level::Settings, message);
}

/// Read all lines from the log file.
pub fn read_log_lines() -> Vec<String> {
    let path = get_log_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => content.lines().map(|l| l.to_string()).collect(),
        Err(_) => vec!["(No log file found)".to_string()],
    }
}
