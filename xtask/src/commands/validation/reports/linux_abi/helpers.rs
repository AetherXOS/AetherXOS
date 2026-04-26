use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;

#[derive(Serialize)]
pub(crate) struct SyscallFamilyTier {
    pub(crate) family: String,
    pub(crate) total: usize,
    pub(crate) implemented: usize,
    pub(crate) partial: usize,
    pub(crate) no: usize,
    pub(crate) external: usize,
    pub(crate) coverage_pct: f64,
    pub(crate) tier: String,
}

#[derive(Deserialize)]
pub(crate) struct SyscallCoverageRow {
    pub(crate) linux_nr: String,
    pub(crate) handler: String,
    pub(crate) status: String,
    pub(crate) reason: String,
}

pub(crate) fn read_json(path: std::path::PathBuf) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub(crate) fn read_syscall_rows(path: &std::path::Path) -> Result<Vec<SyscallCoverageRow>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed reading syscall coverage rows: {}", path.display()))?;
    serde_json::from_str(&text)
        .with_context(|| format!("failed parsing syscall coverage rows: {}", path.display()))
}

pub(crate) fn infer_family(linux_nr: &str) -> &'static str {
    if linux_nr.starts_with("EPOLL") || linux_nr.starts_with("POLL") || linux_nr.starts_with("SELECT") {
        "event"
    } else if linux_nr.contains("SOCKET")
        || linux_nr.starts_with("BIND")
        || linux_nr.starts_with("CONNECT")
        || linux_nr.starts_with("ACCEPT")
        || linux_nr.starts_with("SEND")
        || linux_nr.starts_with("RECV")
    {
        "network"
    } else if linux_nr.starts_with("FUTEX")
        || linux_nr.starts_with("CLONE")
        || linux_nr.starts_with("EXEC")
        || linux_nr.starts_with("WAIT")
        || linux_nr.starts_with("KILL")
        || linux_nr.starts_with("TKILL")
        || linux_nr.starts_with("TGKILL")
    {
        "process"
    } else if linux_nr.contains("TIME")
        || linux_nr.starts_with("CLOCK")
        || linux_nr.starts_with("NANOSLEEP")
        || linux_nr.starts_with("TIMER")
    {
        "time"
    } else if linux_nr.starts_with("OPEN")
        || linux_nr.starts_with("CLOSE")
        || linux_nr.starts_with("READ")
        || linux_nr.starts_with("WRITE")
        || linux_nr.starts_with("STAT")
        || linux_nr.starts_with("FSTAT")
        || linux_nr.starts_with("LSEEK")
        || linux_nr.starts_with("FCNTL")
        || linux_nr.starts_with("DUP")
        || linux_nr.starts_with("IOCTL")
    {
        "fd_fs"
    } else if linux_nr.starts_with("MMAP")
        || linux_nr.starts_with("MUNMAP")
        || linux_nr.starts_with("MPROTECT")
        || linux_nr.starts_with("BRK")
        || linux_nr.starts_with("MREMAP")
    {
        "memory"
    } else {
        "misc"
    }
}

pub(crate) fn compute_family_tiers(rows: &[SyscallCoverageRow]) -> Vec<SyscallFamilyTier> {
    #[derive(Default)]
    struct FamilyStats {
        total: usize,
        implemented: usize,
        partial: usize,
        no: usize,
        external: usize,
    }

    let mut stats: BTreeMap<String, FamilyStats> = BTreeMap::new();
    for row in rows {
        let family = infer_family(&row.linux_nr).to_string();
        let entry = stats.entry(family).or_default();
        entry.total += 1;
        match row.status.as_str() {
            "implemented" => entry.implemented += 1,
            "partial" => entry.partial += 1,
            "no" => entry.no += 1,
            "external" => entry.external += 1,
            _ => entry.external += 1,
        }
    }

    let mut out = Vec::new();
    for (family, st) in stats {
        let weighted_ok = st.implemented as f64 + (st.partial as f64 * 0.5);
        let coverage_pct = if st.total == 0 {
            0.0
        } else {
            ((weighted_ok / st.total as f64) * 1000.0).round() / 10.0
        };
        let tier = if coverage_pct >= 95.0 {
            "ga"
        } else if coverage_pct >= 80.0 {
            "beta"
        } else {
            "alpha"
        };
        out.push(SyscallFamilyTier {
            family,
            total: st.total,
            implemented: st.implemented,
            partial: st.partial,
            no: st.no,
            external: st.external,
            coverage_pct,
            tier: tier.to_string(),
        });
    }
    out
}

pub(crate) fn suggest_alternative(row: &SyscallCoverageRow) -> String {
    let name = row.linux_nr.as_str();
    if name.starts_with("IO_URING") {
        "fallback to epoll + nonblocking read/write".to_string()
    } else if name.starts_with("BPF") {
        "use static policy hooks and userspace filtering".to_string()
    } else if name.starts_with("KEXEC") {
        "prefer reboot path managed by bootloader slot handoff".to_string()
    } else if name.starts_with("MOUNT") || name.starts_with("UMOUNT") {
        "use pre-mounted image layout or initramfs bundling".to_string()
    } else if name.starts_with("FANOTIFY") || name.starts_with("INOTIFY") {
        "use periodic metadata polling and event bridge".to_string()
    } else if row.status == "partial" {
        "supported with reduced semantics; validate edge cases in compat tests".to_string()
    } else {
        "consider POSIX-compatible wrapper in userspace shim".to_string()
    }
}
