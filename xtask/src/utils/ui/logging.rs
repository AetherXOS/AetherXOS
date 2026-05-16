use anyhow::Result;
use chrono::Local;
use colored::*;
use serde_json::{Map, json};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Write;
use std::sync::Mutex;
use once_cell::sync::Lazy;

use super::orchestrator::MULTI_PROGRESS;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Status = 3,
    Success = 4,
    Warn = 5,
    Error = 6,
}

static CURRENT_LEVEL: Lazy<Mutex<LogLevel>> = Lazy::new(|| Mutex::new(LogLevel::Info));
static LOG_FILE: Lazy<Mutex<Option<File>>> = Lazy::new(|| Mutex::new(None));

pub fn init_logger(level: LogLevel, log_to_file: bool) -> Result<()> {
    *CURRENT_LEVEL.lock().unwrap() = level;
    
    if log_to_file {
        let log_dir = crate::utils::paths::resolve("artifacts/logs");
        crate::utils::paths::ensure_dir(&log_dir)?;
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let log_path = log_dir.join(format!("xtask_{}.log", timestamp));
        let file = File::create(log_path)?;
        *LOG_FILE.lock().unwrap() = Some(file);
    }
    
    Ok(())
}

pub fn print_header(about: &str, system: &str, target: &str) {
    let width = 60;
    let bar = "━".repeat(width).bright_black();
    
    let header = format!(
        "\n  {}\n  {}  {} {}\n  {}  {} {}\n  {}  {} {}\n  {}\n",
        bar,
        "✨".yellow(),
        "Project:".bold(),
        about.white().bold(),
        "💻".blue(),
        "System :".bold(),
        system.dimmed(),
        "🎯".magenta(),
        "Target :".bold(),
        target.dimmed(),
        bar
    );
    let _ = MULTI_PROGRESS.println(header);
}

fn get_timestamp() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

pub fn log(level_str: &str, module: &str, message: &str, kv: &[(&str, &str)]) {
    let level = match level_str {
        "TRACE" => LogLevel::Trace,
        "DEBUG" => LogLevel::Debug,
        "INFO"  => LogLevel::Info,
        "READY" => LogLevel::Success,
        "WARN"  => LogLevel::Warn,
        "ERROR" => LogLevel::Error,
        _       => LogLevel::Info,
    };

    if level < *CURRENT_LEVEL.lock().unwrap() {
        return;
    }

    // 1. File Logging
    if let Some(mut file) = LOG_FILE.lock().unwrap().as_ref() {
        let ts = Local::now().to_rfc3339();
        let kv_str = kv.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join(" ");
        let log_line = format!("[{}] [{}] [{}] {} {}\n", ts, level_str, module, message, kv_str);
        let _ = file.write_all(log_line.as_bytes());
    }

    if std::env::var("XTASK_LOG_JSON").is_ok() {
        // ... (existing JSON logic)
        let mut fields = Map::new();
        for (k, v) in kv {
            fields.insert(k.to_string(), json!(v));
        }
        let record = json!({
            "ts": Local::now().to_rfc3339(),
            "level": level_str,
            "module": module,
            "message": message,
            "fields": fields
        });
        MULTI_PROGRESS.println(record.to_string()).ok();
        return;
    }

    let ts = get_timestamp().dimmed();
    let lvl_styled = match level_str {
        "ERROR" => " ERROR ".on_red().white().bold(),
        "WARN"  => "  WARN ".on_yellow().black().bold(),
        "EXEC"  => "  EXEC ".on_magenta().white().bold(),
        "READY" => " READY ".on_green().black().bold(),
        "STEP"  => "  STEP ".on_cyan().black().bold(),
        _       => "  INFO ".on_blue().white().bold(),
    };

    let mod_styled = format!(" {:<8} ", module).on_bright_black().white();

    let main_line = format!("{} {} {} {}", ts, lvl_styled, mod_styled, message.white().bold());
    MULTI_PROGRESS.println(main_line).ok();

    if !kv.is_empty() {
        for (k, v) in kv {
            let kv_line = format!("         {} {} {}: {}", "│".dimmed(), "▹".dimmed(), k.dimmed(), v.cyan());
            MULTI_PROGRESS.println(kv_line).ok();
        }
    }
}

pub fn info(module: &str, message: &str, kv: &[(&str, &str)]) {
    log("INFO", module, message, kv);
}
pub fn debug(module: &str, message: &str, kv: &[(&str, &str)]) {
    log("DEBUG", module, message, kv);
}
pub fn trace(module: &str, message: &str, kv: &[(&str, &str)]) {
    log("TRACE", module, message, kv);
}
pub fn warn(module: &str, message: &str, kv: &[(&str, &str)]) {
    log("WARN", module, message, kv);
}
pub fn exec(module: &str, command: &str) {
    log("EXEC", module, command, &[]);
}
pub fn error(module: &str, message: &str, kv: &[(&str, &str)]) {
    log("ERROR", module, message, kv);
}
pub fn success(module: &str, message: &str, kv: &[(&str, &str)]) {
    log("READY", module, message, kv);
}
pub fn step(module: &str, message: &str) {
    log("STEP", module, message, &[]);
}

pub trait ReadyDetails {
    fn log_ready(self, module: &str, message: &str);
}

impl ReadyDetails for &Path {
    fn log_ready(self, module: &str, message: &str) {
        log(
            "READY",
            module,
            message,
            &[("path", &self.to_string_lossy())],
        );
    }
}

impl ReadyDetails for PathBuf {
    fn log_ready(self, module: &str, message: &str) {
        log(
            "READY",
            module,
            message,
            &[("path", &self.to_string_lossy())],
        );
    }
}

impl ReadyDetails for &str {
    fn log_ready(self, module: &str, message: &str) {
        log("READY", module, message, &[("path", self)]);
    }
}

impl ReadyDetails for String {
    fn log_ready(self, module: &str, message: &str) {
        log("READY", module, message, &[("path", &self)]);
    }
}

impl ReadyDetails for &String {
    fn log_ready(self, module: &str, message: &str) {
        log("READY", module, message, &[("path", self)]);
    }
}

impl ReadyDetails for Cow<'_, str> {
    fn log_ready(self, module: &str, message: &str) {
        log("READY", module, message, &[("path", &self)]);
    }
}

impl ReadyDetails for &[(&str, &str)] {
    fn log_ready(self, module: &str, message: &str) {
        log("READY", module, message, self);
    }
}

impl<const N: usize> ReadyDetails for &[(&str, &str); N] {
    fn log_ready(self, module: &str, message: &str) {
        log("READY", module, message, self);
    }
}

impl<const N: usize> ReadyDetails for &[(&str, &String); N] {
    fn log_ready(self, module: &str, message: &str) {
        let kv: Vec<(&str, &str)> = self.iter().map(|(k, v)| (*k, v.as_str())).collect();
        log("READY", module, message, &kv);
    }
}

impl ReadyDetails for &Cow<'_, str> {
    fn log_ready(self, module: &str, message: &str) {
        log("READY", module, message, &[("path", self)]);
    }
}

pub fn ready<D: ReadyDetails>(module: &str, message: &str, details: D) {
    details.log_ready(module, message);
}

pub fn status(module: &str, message: &str) {
    let ts = get_timestamp().dimmed();
    let symbol = "◈".bright_blue().bold();
    let mod_styled = format!("[{}]", module).bright_black();
    let _ = MULTI_PROGRESS.println(format!("{}  {} {} {}", ts, symbol, mod_styled, message.white()));
}

pub fn progress(module: &str, message: &str, percent: u32) {
    let ts = get_timestamp().dimmed();
    let symbol = "⚙".magenta().bold();
    let bar_width = 20;
    let filled = (percent as f32 / 100.0 * bar_width as f32) as usize;
    let empty = bar_width - filled;
    let bar = format!("{}{}", "█".repeat(filled).magenta(), "░".repeat(empty).bright_black());
    let _ = MULTI_PROGRESS.println(format!("{}  {} {:<8} {} {} {}%", ts, symbol, module, bar, message.dimmed(), percent));
}
