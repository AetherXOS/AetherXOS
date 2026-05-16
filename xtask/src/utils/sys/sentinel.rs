use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::process::Child;
use crate::utils::logging;

/// Global tracker for child processes to prevent orphans.
static TRACKER: Lazy<Mutex<Vec<u32>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub struct Sentinel;

impl Sentinel {
    /// Register a child process for tracking.
    pub fn track(child: &Child) {
        if let Ok(mut pids) = TRACKER.lock() {
            pids.push(child.id());
        }
    }

    /// Kill all tracked child processes.
    pub fn cleanup() {
        if let Ok(mut pids) = TRACKER.lock() {
            if pids.is_empty() { return; }
            logging::warn("SENTINEL", &format!("Cleaning up {} orphan processes...", pids.len()), &[]);
            
            for pid in pids.drain(..) {
                Self::kill_pid(pid);
            }
        }
    }

    /// Initialize the global signal handler for Ctrl+C.
    pub fn init_signals() {
        ctrlc::set_handler(move || {
            println!("\n\x1b[31;1m[!] Termination signal received. Sentinel engaging...\x1b[0m");
            Self::cleanup();
            crate::utils::ui::logging::shutdown_logger();
            std::process::exit(130);
        }).expect("Failed to initialize Sentinel signal handler");
    }

    fn kill_pid(pid: u32) {
        if cfg!(windows) {
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .status();
        } else {
            let _ = std::process::Command::new("kill")
                .arg("-9")
                .arg(&pid.to_string())
                .status();
        }
    }
}
