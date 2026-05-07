use chrono::Local;
use colored::*;
use serde_json::{Map, json};
use std::borrow::Cow;
use std::path::{Path, PathBuf};

use super::orchestrator::MULTI_PROGRESS;

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

pub fn log(level: &str, module: &str, message: &str, kv: &[(&str, &str)]) {
    if std::env::var("XTASK_LOG_JSON").is_ok() {
        let mut fields = Map::new();
        for (k, v) in kv {
            fields.insert(k.to_string(), json!(v));
        }
        let record = json!({
            "ts": Local::now().to_rfc3339(),
            "level": level,
            "module": module,
            "message": message,
            "fields": fields
        });
        MULTI_PROGRESS.println(record.to_string()).ok();
        return;
    }

    let ts = get_timestamp().dimmed();
    let lvl_styled = match level {
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
    let _ = MULTI_PROGRESS.println(format!("  {} {} {}", "→".blue().bold(), module.dimmed(), message));
}
