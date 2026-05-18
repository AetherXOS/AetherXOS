pub mod background;
pub mod types;

use background::{LOG_TX, LogCommand, init_background_writer};
use once_cell::sync::Lazy;
use std::sync::Mutex;
pub use types::LogLevel;

pub static CURRENT_TASK: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

pub fn init_logger(_level: LogLevel, to_file: bool) -> anyhow::Result<()> {
    if to_file {
        let path = std::path::Path::new("artifacts/xtask.log");
        let tx = init_background_writer(path);
        if let Ok(mut global_tx) = LOG_TX.lock() {
            *global_tx = Some(tx);
        }
    }
    Ok(())
}

pub fn log(level: &str, tag: &str, msg: &str) {
    let color_code = match level {
        "INFO" => "\x1b[34m",    // Blue
        "WARN" => "\x1b[33m",    // Yellow
        "ERROR" => "\x1b[31m",   // Red
        "SUCCESS" => "\x1b[32m", // Green
        "READY" => "\x1b[35m",   // Magenta
        "STATUS" => "\x1b[36m",  // Cyan
        "STEP" => "\x1b[90m",    // Gray
        "EXEC" => "\x1b[94m",    // Bright Blue
        "DEBUG" => "\x1b[90m",   // Gray
        "AOP" => "\x1b[95m",     // Bright Magenta
        _ => "\x1b[0m",
    };
    let reset = "\x1b[0m";
    let formatted = format!("{}[{}] [{}] {}{}", color_code, level, tag, msg, reset);
    println!("{}", formatted);

    // For file logging, strip ANSI
    let raw_formatted = format!("[{}] [{}] {}", level, tag, msg);
    if let Ok(tx_opt) = LOG_TX.lock() {
        if let Some(tx) = tx_opt.as_ref() {
            let _ = tx.send(LogCommand::Write(raw_formatted));
        }
    }
}

pub fn aop_wrap<T, F: FnOnce() -> T>(tag: &str, action: &str, f: F) -> T {
    log("AOP", tag, &format!("Enter: {}", action));
    let start = std::time::Instant::now();
    let result = f();
    log(
        "AOP",
        tag,
        &format!("Exit: {} (took {:?})", action, start.elapsed()),
    );
    result
}

pub fn aop_wrap_result<T, E, F: FnOnce() -> Result<T, E>>(
    tag: &str,
    action: &str,
    f: F,
) -> Result<T, E>
where
    E: std::fmt::Display,
{
    log("AOP", tag, &format!("Enter: {}", action));
    let start = std::time::Instant::now();
    match f() {
        Ok(val) => {
            log(
                "AOP",
                tag,
                &format!("Success: {} (took {:?})", action, start.elapsed()),
            );
            Ok(val)
        }
        Err(e) => {
            log(
                "AOP",
                tag,
                &format!(
                    "Failure: {} - Error: {} (took {:?})",
                    action,
                    e,
                    start.elapsed()
                ),
            );
            Err(e)
        }
    }
}

pub fn info(tag: &str, msg: &str, fields: &[(&str, &str)]) {
    log("INFO", tag, msg);
    for (k, v) in fields {
        println!("  -> {}: {}", k, v);
    }
}
pub fn warn(tag: &str, msg: &str, fields: &[(&str, &str)]) {
    log("WARN", tag, msg);
    for (k, v) in fields {
        println!("  -> {}: {}", k, v);
    }
}
pub fn error(tag: &str, msg: &str, fields: &[(&str, &str)]) {
    log("ERROR", tag, msg);
    for (k, v) in fields {
        println!("  -> {}: {}", k, v);
    }
}
pub fn success(tag: &str, msg: &str, fields: &[(&str, &str)]) {
    log("SUCCESS", tag, msg);
    for (k, v) in fields {
        println!("  -> {}: {}", k, v);
    }
}
pub fn ready(tag: &str, msg: &str, path: &str, _fields: &[(&str, &str)]) {
    log("READY", tag, &format!("{}: {}", msg, path));
}
pub fn status(tag: &str, msg: &str) {
    log("STATUS", tag, msg);
}
pub fn step(tag: &str, msg: &str) {
    log("STEP", tag, msg);
}
pub fn exec(tag: &str, cmd: &str) {
    log("EXEC", tag, cmd);
}
pub fn debug(tag: &str, msg: &str, _fields: &[(&str, &str)]) {
    log("DEBUG", tag, msg);
}

pub fn set_current_task(name: Option<String>) {
    if let Ok(mut task) = CURRENT_TASK.lock() {
        *task = name;
    }
}

pub fn shutdown_logger() {
    if let Ok(mut tx) = LOG_TX.lock() {
        if let Some(tx) = tx.take() {
            let _ = tx.send(LogCommand::Flush);
            let _ = tx.send(LogCommand::Terminate);
        }
    }
}

pub fn print_header(about: &str, system: &str, target: &str) {
    println!("=== {} ===", about);
    println!("System: {}", system);
    println!("Target: {}", target);
    println!("==========================");
}
