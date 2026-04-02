//! Linux Compatibility Feature Inspection & Capability Reporting.
//!
//! Provides compile-time and runtime introspection of which Linux
//! compatibility features are available in this kernel build.
//! Useful for:
//! - Applications querying capabilities via /proc
//! - Debug/boot logging
//! - Feature-gated code paths
//! - CI/CD build validation

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// A single feature capability entry.
#[derive(Debug, Clone)]
pub struct FeatureEntry {
    pub name: &'static str,
    pub category: FeatureCategory,
    pub enabled: bool,
    pub description: &'static str,
}

/// Feature categories for grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureCategory {
    VirtualFS,
    IPC,
    Process,
    Memory,
    Network,
    Security,
    Scheduler,
    Device,
}

impl FeatureCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VirtualFS => "Virtual Filesystem",
            Self::IPC => "Inter-Process Communication",
            Self::Process => "Process Management",
            Self::Memory => "Memory Management",
            Self::Network => "Networking",
            Self::Security => "Security",
            Self::Scheduler => "Scheduler",
            Self::Device => "Device Drivers",
        }
    }
}

/// Build a comprehensive list of all Linux compatibility features
/// and their compile-time enabled/disabled status.
pub fn feature_inventory() -> Vec<FeatureEntry> {
    let mut features = Vec::with_capacity(64);

    // ── Virtual Filesystem ──────────────────────────────────────────

    features.push(FeatureEntry {
        name: "procfs",
        category: FeatureCategory::VirtualFS,
        enabled: cfg!(feature = "linux_compat"),
        description: "/proc virtual filesystem",
    });

    features.push(FeatureEntry {
        name: "sysfs",
        category: FeatureCategory::VirtualFS,
        enabled: cfg!(feature = "linux_compat"),
        description: "/sys virtual filesystem",
    });

    features.push(FeatureEntry {
        name: "tmpfs",
        category: FeatureCategory::VirtualFS,
        enabled: true, // always available
        description: "In-memory temporary filesystem",
    });

    features.push(FeatureEntry {
        name: "devpts",
        category: FeatureCategory::VirtualFS,
        enabled: cfg!(feature = "linux_compat"),
        description: "Pseudo-terminal filesystem (/dev/pts)",
    });

    features.push(FeatureEntry {
        name: "dev_special",
        category: FeatureCategory::Device,
        enabled: true,
        description: "/dev/null, /dev/zero, /dev/random, /dev/tty",
    });

    features.push(FeatureEntry {
        name: "ramfs",
        category: FeatureCategory::VirtualFS,
        enabled: cfg!(feature = "vfs_ramfs"),
        description: "RAM-based filesystem",
    });

    features.push(FeatureEntry {
        name: "ext4",
        category: FeatureCategory::VirtualFS,
        enabled: cfg!(feature = "vfs_ext4"),
        description: "EXT4 filesystem (read-only)",
    });

    features.push(FeatureEntry {
        name: "fatfs",
        category: FeatureCategory::VirtualFS,
        enabled: cfg!(feature = "vfs_fatfs"),
        description: "FAT32 filesystem",
    });

    // ── IPC ──────────────────────────────────────────────────────────

    features.push(FeatureEntry {
        name: "pipe",
        category: FeatureCategory::IPC,
        enabled: cfg!(feature = "posix_pipe"),
        description: "POSIX pipes (pipe, pipe2)",
    });

    features.push(FeatureEntry {
        name: "unix_domain_socket",
        category: FeatureCategory::IPC,
        enabled: cfg!(feature = "ipc_unix_domain"),
        description: "Unix domain sockets (AF_UNIX)",
    });

    features.push(FeatureEntry {
        name: "futex",
        category: FeatureCategory::IPC,
        enabled: cfg!(feature = "ipc_futex"),
        description: "Fast userspace mutex (futex)",
    });

    features.push(FeatureEntry {
        name: "eventfd",
        category: FeatureCategory::IPC,
        enabled: cfg!(feature = "posix_io"),
        description: "Event file descriptors (eventfd, eventfd2)",
    });

    features.push(FeatureEntry {
        name: "signalfd",
        category: FeatureCategory::IPC,
        enabled: cfg!(feature = "posix_signal"),
        description: "Signal file descriptors (signalfd, signalfd4)",
    });

    features.push(FeatureEntry {
        name: "timerfd",
        category: FeatureCategory::IPC,
        enabled: cfg!(feature = "posix_time"),
        description: "Timer file descriptors",
    });

    features.push(FeatureEntry {
        name: "sysv_semaphore",
        category: FeatureCategory::IPC,
        enabled: cfg!(feature = "ipc_sysv_sem"),
        description: "System V semaphores",
    });

    features.push(FeatureEntry {
        name: "sysv_msgqueue",
        category: FeatureCategory::IPC,
        enabled: cfg!(feature = "ipc_sysv_msg"),
        description: "System V message queues",
    });

    // ── Process ─────────────────────────────────────────────────────

    features.push(FeatureEntry {
        name: "process_abstraction",
        category: FeatureCategory::Process,
        enabled: cfg!(feature = "process_abstraction"),
        description: "Process/thread abstraction layer",
    });

    features.push(FeatureEntry {
        name: "posix_signal",
        category: FeatureCategory::Process,
        enabled: cfg!(feature = "posix_signal"),
        description: "POSIX signal handling (sigaction, kill, etc.)",
    });

    features.push(FeatureEntry {
        name: "posix_thread",
        category: FeatureCategory::Process,
        enabled: cfg!(feature = "posix_thread"),
        description: "POSIX threading (clone, pthread)",
    });

    // ── Memory ──────────────────────────────────────────────────────

    features.push(FeatureEntry {
        name: "mmap",
        category: FeatureCategory::Memory,
        enabled: cfg!(feature = "posix_mman"),
        description: "Memory mapping (mmap, munmap, mprotect)",
    });

    features.push(FeatureEntry {
        name: "paging",
        category: FeatureCategory::Memory,
        enabled: cfg!(feature = "paging_enable"),
        description: "Virtual memory paging (MMU)",
    });

    // ── Network ─────────────────────────────────────────────────────

    features.push(FeatureEntry {
        name: "networking",
        category: FeatureCategory::Network,
        enabled: cfg!(feature = "networking"),
        description: "Network stack (TCP/UDP/IP)",
    });

    features.push(FeatureEntry {
        name: "epoll",
        category: FeatureCategory::Network,
        enabled: cfg!(feature = "posix_net"),
        description: "Event poll (epoll_create, epoll_ctl, epoll_wait)",
    });

    // ── Security ────────────────────────────────────────────────────

    features.push(FeatureEntry {
        name: "security",
        category: FeatureCategory::Security,
        enabled: cfg!(feature = "security"),
        description: "Security module framework",
    });

    features.push(FeatureEntry {
        name: "capabilities",
        category: FeatureCategory::Security,
        enabled: cfg!(feature = "capabilities"),
        description: "Linux capabilities (CAP_*)",
    });

    features.push(FeatureEntry {
        name: "ring_protection",
        category: FeatureCategory::Security,
        enabled: cfg!(feature = "ring_protection"),
        description: "Ring 0/3 separation with syscalls",
    });

    // ── Scheduler ───────────────────────────────────────────────────

    features.push(FeatureEntry {
        name: "cfs",
        category: FeatureCategory::Scheduler,
        enabled: cfg!(feature = "sched_cfs"),
        description: "Completely Fair Scheduler",
    });

    features.push(FeatureEntry {
        name: "edf",
        category: FeatureCategory::Scheduler,
        enabled: cfg!(feature = "sched_edf"),
        description: "Earliest Deadline First scheduler",
    });

    features
}

/// Generate a human-readable feature report string.
pub fn feature_report() -> String {
    let features = feature_inventory();
    let mut report = String::with_capacity(2048);

    report.push_str("═══ AetherCore Linux Compatibility Features ═══\n\n");

    let categories = [
        FeatureCategory::VirtualFS,
        FeatureCategory::IPC,
        FeatureCategory::Process,
        FeatureCategory::Memory,
        FeatureCategory::Network,
        FeatureCategory::Security,
        FeatureCategory::Scheduler,
        FeatureCategory::Device,
    ];

    for cat in &categories {
        let cat_features: Vec<_> = features.iter().filter(|f| f.category == *cat).collect();
        if cat_features.is_empty() {
            continue;
        }

        report.push_str(&alloc::format!("── {} ──\n", cat.as_str()));
        for f in &cat_features {
            let status = if f.enabled { "✓" } else { "✗" };
            report.push_str(&alloc::format!(
                "  [{}] {:<25} {}\n",
                status, f.name, f.description
            ));
        }
        report.push('\n');
    }

    let enabled = features.iter().filter(|f| f.enabled).count();
    let total = features.len();
    report.push_str(&alloc::format!(
        "Total: {}/{} features enabled ({:.0}%)\n",
        enabled,
        total,
        (enabled as f64 / total as f64) * 100.0
    ));

    report
}

/// Count of enabled features at compile time.
pub fn enabled_feature_count() -> usize {
    feature_inventory().iter().filter(|f| f.enabled).count()
}

/// Check if a specific feature is available.
pub fn has_feature(name: &str) -> bool {
    feature_inventory().iter().any(|f| f.name == name && f.enabled)
}

/// Feature summary for /proc/aethercore/features exposure.
pub fn features_procfs_string() -> String {
    let features = feature_inventory();
    let mut result = String::with_capacity(512);
    for f in &features {
        if f.enabled {
            result.push_str(f.name);
            result.push('\n');
        }
    }
    result
}
