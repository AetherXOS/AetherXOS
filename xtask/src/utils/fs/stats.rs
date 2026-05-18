use crate::utils::config;
use crossbeam_channel::{Receiver, unbounded};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{Clear, ClearType},
};
use std::fs;
use std::io::{Write, stdout};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// Represents the collected statistics for a directory tree.
#[derive(Debug, Clone, Default)]
pub struct DirStats {
    pub file_count: u64,
    pub total_bytes: u64,
    pub interrupted: bool,
}

impl DirStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge(&mut self, other: &DirStats) {
        self.file_count += other.file_count;
        self.total_bytes += other.total_bytes;
        if other.interrupted {
            self.interrupted = true;
        }
    }

    pub fn format_size(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if self.total_bytes >= GB {
            format!("{:.2} GB", self.total_bytes as f64 / GB as f64)
        } else if self.total_bytes >= MB {
            format!("{:.2} MB", self.total_bytes as f64 / MB as f64)
        } else if self.total_bytes >= KB {
            format!("{:.2} KB", self.total_bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", self.total_bytes)
        }
    }
}

/// Orchestrates the background scanning and user interaction.
pub struct ScanOrchestrator {
    timeout: Duration,
    cumulative_start: Instant,
    stop_signal: Arc<AtomicBool>,
}

impl ScanOrchestrator {
    pub fn new(total_timeout_secs: u64) -> Self {
        Self {
            timeout: Duration::from_secs(total_timeout_secs),
            cumulative_start: Instant::now(),
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Executes a scan on a specific path, respecting the cumulative timeout.
    pub fn scan<P: AsRef<Path>>(&self, path: P) -> DirStats {
        let path = path.as_ref().to_path_buf();
        let (tx, rx) = unbounded();
        let stop_signal_worker = Arc::clone(&self.stop_signal);
        let scan_path = path.clone();

        // If already interrupted, just return empty
        if self.stop_signal.load(Ordering::SeqCst) {
            let mut s = DirStats::new();
            s.interrupted = true;
            return s;
        }

        // Spawn worker thread for background scanning
        thread::spawn(move || {
            let stats = ScanOrchestrator::scan_recursive(&scan_path, &stop_signal_worker);
            let _ = tx.send(stats);
        });

        loop {
            // 1. Check if calculation finished
            if let Ok(stats) = rx.try_recv() {
                return stats;
            }

            // 2. Check if we hit the cumulative threshold
            if self.cumulative_start.elapsed() > self.timeout {
                if config::is_non_interactive() {
                    // In non-interactive mode, we just wait (or we could skip if it's too long)
                    thread::sleep(Duration::from_millis(100));
                    continue;
                } else {
                    return self.enter_interaction_gate(rx);
                }
            }

            thread::sleep(Duration::from_millis(50));
        }
    }

    /// Enters a non-blocking interactive loop to allow users to skip or wait.
    fn enter_interaction_gate(&self, rx: Receiver<DirStats>) -> DirStats {
        let _ = write!(
            stdout(),
            "\r[STATS] Calculation taking longer than expected. [S]kip or wait? "
        );
        let _ = stdout().flush();

        loop {
            // 1. Monitor background task
            if let Ok(stats) = rx.try_recv() {
                self.clear_gate_line();
                return stats;
            }

            // 2. Poll user input
            if let Ok(true) = event::poll(Duration::from_millis(50)) {
                if let Ok(Event::Key(key)) = event::read() {
                    match key.code {
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            self.stop_signal.store(true, Ordering::SeqCst);
                            self.clear_gate_line();
                            println!("[INFO] Stats calculation skipped by user.");
                            let mut stats = DirStats::new();
                            stats.interrupted = true;
                            return stats;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn clear_gate_line(&self) {
        let mut out = stdout();
        let _ = execute!(out, Clear(ClearType::CurrentLine));
        let _ = write!(out, "\r");
        let _ = out.flush();
    }

    fn scan_recursive(path: &Path, stop: &AtomicBool) -> DirStats {
        let mut stats = DirStats::new();

        if stop.load(Ordering::SeqCst) {
            stats.interrupted = true;
            return stats;
        }

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if stop.load(Ordering::SeqCst) {
                    stats.interrupted = true;
                    return stats;
                }

                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        stats.file_count += 1;
                        stats.total_bytes += metadata.len();
                    } else if metadata.is_dir() {
                        let sub_stats = Self::scan_recursive(&entry.path(), stop);
                        stats.merge(&sub_stats);
                        if stats.interrupted {
                            return stats;
                        }
                    }
                }
            }
        }
        stats
    }
}

/// Facade for quick one-off stats (legacy compatibility)
pub fn get_dir_stats<P: AsRef<Path>>(path: P) -> DirStats {
    let orchestrator = ScanOrchestrator::new(3);
    orchestrator.scan(path)
}

pub fn get_file_stats<P: AsRef<Path>>(path: P) -> DirStats {
    let mut stats = DirStats::new();
    if let Ok(metadata) = fs::metadata(path) {
        if metadata.is_file() {
            stats.file_count = 1;
            stats.total_bytes = metadata.len();
        }
    }
    stats
}
