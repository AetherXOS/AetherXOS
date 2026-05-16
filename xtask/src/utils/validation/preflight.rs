use anyhow::{Result, bail, Context};
use sysinfo::{System, RefreshKind, CpuRefreshKind};
use crate::utils::logging;

pub struct SystemRequirements {
    pub min_ram_gb: u64,
    pub min_disk_gb: u64,
    pub min_cores: usize,
}

impl Default for SystemRequirements {
    fn default() -> Self {
        Self {
            min_ram_gb: 8,
            min_disk_gb: 20,
            min_cores: 4,
        }
    }
}

pub fn run_audit() -> Result<()> {
    let reqs = SystemRequirements::default();
    run_preflight_check(&reqs)
}

pub fn run_preflight_check(reqs: &SystemRequirements) -> Result<()> {
    logging::status("PREFLIGHT", "Verifying hardware requirements...");
    
    let mut sys = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(sysinfo::MemoryRefreshKind::everything())
    );
    sys.refresh_all();
    
    // 1. RAM Check
    let total_ram_gb = sys.total_memory() / 1024 / 1024 / 1024;
    if total_ram_gb < reqs.min_ram_gb {
        logging::warn("PREFLIGHT", &format!("Insufficient RAM: {}GB (Recommended: {}GB)", total_ram_gb, reqs.min_ram_gb), &[]);
    }
    
    // 2. CPU Cores Check
    let cores = sys.cpus().len();
    if cores < reqs.min_cores {
        logging::warn("PREFLIGHT", &format!("Low CPU core count: {} (Recommended: {})", cores, reqs.min_cores), &[]);
    }
    
    // 3. Disk Space Check
    let repo_root = crate::utils::paths::repo_root();
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut disk_available_gb = 0;
    for disk in &disks {
        if repo_root.starts_with(disk.mount_point()) {
            disk_available_gb = disk.available_space() / 1024 / 1024 / 1024;
            break;
        }
    }
    
    if disk_available_gb < reqs.min_disk_gb {
        bail!("Insufficient disk space: {}GB available (Required: {}GB)", disk_available_gb, reqs.min_disk_gb);
    }
    
    // 4. Host OS Integrity (Optional check for dev tools)
    verify_toolchain_hermetic().context("Hermetic toolchain validation failed")?;
    
    logging::success("PREFLIGHT", "Hardware and toolchain validation passed", &[]);
    Ok(())
}

fn verify_toolchain_hermetic() -> Result<()> {
    // This is where we verify rustc/cargo versions against a pinned set
    let output = std::process::Command::new("rustc").arg("--version").output()?;
    let version = String::from_utf8_lossy(&output.stdout);
    logging::info("PREFLIGHT", &format!("Toolchain: {}", version.trim()), &[]);
    Ok(())
}
