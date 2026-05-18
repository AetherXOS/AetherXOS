use std::process::Command;

#[derive(Debug, Clone)]
pub struct NotificationTelemetry {
    pub os_name: String,
    pub cpu_cores: usize,
    pub total_ram: String,
    pub kernel_size: String,
    pub git_hash: Option<String>,
    pub git_author: Option<String>,
    pub git_message: Option<String>,
}

impl NotificationTelemetry {
    /// Gathers dynamic host telemetry and workspace git statistics cleanly
    pub fn gather() -> Self {
        let (git_hash, git_author, git_message) = Self::get_git_info();
        let (cpu_cores, total_ram) = Self::get_sys_telemetry();
        let kernel_size = Self::get_kernel_size().unwrap_or_else(|| "N/A".to_string());
        
        Self {
            os_name: std::env::consts::OS.to_string(),
            cpu_cores,
            total_ram,
            kernel_size,
            git_hash,
            git_author,
            git_message,
        }
    }

    fn run_git_cmd(args: &[&str]) -> Option<String> {
        let output = Command::new("git")
            .args(args)
            .output()
            .ok()?;
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() { Some(s) } else { None }
        } else {
            None
        }
    }

    fn get_git_info() -> (Option<String>, Option<String>, Option<String>) {
        let hash = Self::run_git_cmd(&["rev-parse", "--short", "HEAD"]);
        let author = Self::run_git_cmd(&["log", "-1", "--pretty=%an"]);
        let msg = Self::run_git_cmd(&["log", "-1", "--pretty=%B"]);
        (hash, author, msg)
    }

    fn get_kernel_size() -> Option<String> {
        if let Some(elf_path) = crate::utils::core::paths::WorkspacePaths::find_kernel_elf() {
            if let Ok(metadata) = std::fs::metadata(elf_path) {
                return Some(crate::utils::fs::format::format_size(metadata.len()));
            }
        }
        None
    }

    fn get_sys_telemetry() -> (usize, String) {
        use sysinfo::System;
        let mut sys = System::new_all();
        sys.refresh_all();
        let cores = sys.cpus().len();
        let mem_str = crate::utils::fs::format::format_size(sys.total_memory());
        (cores, mem_str)
    }
}
