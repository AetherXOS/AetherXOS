use std::fs;
use std::path::Path;

pub struct DirStats {
    pub file_count: u64,
    pub total_bytes: u64,
}

impl DirStats {
    pub fn new() -> Self {
        Self { file_count: 0, total_bytes: 0 }
    }

    pub fn add(&mut self, other: &DirStats) {
        self.file_count += other.file_count;
        self.total_bytes += other.total_bytes;
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

pub fn get_dir_stats<P: AsRef<Path>>(path: P) -> DirStats {
    let mut stats = DirStats::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    stats.file_count += 1;
                    stats.total_bytes += metadata.len();
                } else if metadata.is_dir() {
                    let sub_stats = get_dir_stats(entry.path());
                    stats.add(&sub_stats);
                }
            }
        }
    }
    stats
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
