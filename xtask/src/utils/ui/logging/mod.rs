pub mod background;
pub mod types;

use std::sync::Mutex;
use once_cell::sync::Lazy;
use background::{LOG_TX, LogCommand, init_background_writer};
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
    let formatted = format!("[{}] [{}] {}", level, tag, msg);
    println!("{}", formatted);
    
    if let Ok(tx_opt) = LOG_TX.lock() {
        if let Some(tx) = tx_opt.as_ref() {
            let _ = tx.send(LogCommand::Write(formatted));
        }
    }
}

pub fn info(tag: &str, msg: &str, fields: &[(&str, &str)]) { 
    log("INFO", tag, msg); 
    for (k, v) in fields { println!("  -> {}: {}", k, v); }
}
pub fn warn(tag: &str, msg: &str, fields: &[(&str, &str)]) { 
    log("WARN", tag, msg); 
    for (k, v) in fields { println!("  -> {}: {}", k, v); }
}
pub fn error(tag: &str, msg: &str, fields: &[(&str, &str)]) { 
    log("ERROR", tag, msg); 
    for (k, v) in fields { println!("  -> {}: {}", k, v); }
}
pub fn success(tag: &str, msg: &str, fields: &[(&str, &str)]) { 
    log("SUCCESS", tag, msg); 
    for (k, v) in fields { println!("  -> {}: {}", k, v); }
}
pub fn ready(tag: &str, msg: &str, path: &str, _fields: &[(&str, &str)]) { 
    log("READY", tag, &format!("{}: {}", msg, path)); 
}
pub fn status(tag: &str, msg: &str) { log("STATUS", tag, msg); }
pub fn step(tag: &str, msg: &str) { log("STEP", tag, msg); }
pub fn exec(tag: &str, cmd: &str) { log("EXEC", tag, cmd); }
pub fn debug(tag: &str, msg: &str, _fields: &[(&str, &str)]) { log("DEBUG", tag, msg); }

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
