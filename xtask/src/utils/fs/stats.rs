use crate::utils::config;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use crossbeam_channel::{unbounded, Receiver};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{Clear, ClearType},
    execute,
};
use std::io::{stdout, Write};

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
    path: PathBuf,
    timeout: Duration,
    stop_signal: Arc<AtomicBool>,
}

impl ScanOrchestrator {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            timeout: Duration::from_secs(3),
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Executes the scan and handles potential interactive interruptions.
    pub fn run(self) -> DirStats {
        let (tx, rx) = unbounded();
        let stop_signal_worker = Arc::clone(&self.stop_signal);
        let scan_path = self.path.clone();

        // Spawn worker thread for background scanning
        thread::spawn(move || {
            let stats = ScanOrchestrator::scan_recursive(&scan_path, &stop_signal_worker);
            let _ = tx.send(stats);
        });

        let start = Instant::now();

        loop {
            // Check if calculation finished
            if let Ok(stats) = rx.try_recv() {
                return stats;
            }

            // Check if we hit the threshold
            if start.elapsed() > self.timeout {
                if config::is_non_interactive() {
                    // In non-interactive mode, we just keep waiting
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
        let _ = write!(stdout(), "\r[WARN] Scan latency threshold exceeded. [S]kip or wait? ");
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
                            println!("\n[INFO] Scan aborted by user.");
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

/// Facade function for external access.
pub fn get_dir_stats<P: AsRef<Path>>(path: P) -> DirStats {
    ScanOrchestrator::new(path).run()
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
