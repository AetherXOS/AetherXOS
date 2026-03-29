//! Linux VFS Mount Setup — initializes virtual filesystem mounts during kernel boot.
//!
//! Sets up the standard Linux filesystem hierarchy:
//! - `/dev` (devfs with special devices)
//! - `/proc` (process information)
//! - `/sys` (system information)
//! - `/tmp` (temporary files)
//! - `/dev/pts` (pseudo-terminals)
//! - `/dev/shm` (POSIX shared memory)
//! - `/run` (runtime data)
//!
//! Feature-gated: only builds when `linux_compat` is enabled.
//! Runtime-configurable: individual mounts can be disabled via `LinuxMountConfig`.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::modules::vfs::mount_table::{self, FsType, MountFlags};

// ── Runtime configuration ───────────────────────────────────────────────────

/// Runtime configuration for which virtual filesystems to mount.
/// All fields default to `true` for maximum Linux compatibility.
#[derive(Debug, Clone, Copy)]
pub struct LinuxMountConfig {
    /// Mount /proc (procfs)
    pub mount_proc: bool,
    /// Mount /sys (sysfs)
    pub mount_sys: bool,
    /// Mount /tmp (tmpfs)
    pub mount_tmp: bool,
    /// Mount /dev/pts (pseudo-terminal filesystem)
    pub mount_devpts: bool,
    /// Mount /dev/shm (POSIX shared memory tmpfs)
    pub mount_devshm: bool,
    /// Mount /run (runtime data tmpfs)
    pub mount_run: bool,
    /// Size limit for /tmp in bytes (0 = unlimited)
    pub tmp_size_limit: usize,
    /// Size limit for /dev/shm in bytes (0 = half of RAM)
    pub shm_size_limit: usize,
    /// Size limit for /run in bytes (0 = 20% of RAM)
    pub run_size_limit: usize,
}

impl Default for LinuxMountConfig {
    fn default() -> Self {
        Self {
            mount_proc: true,
            mount_sys: true,
            mount_tmp: true,
            mount_devpts: true,
            mount_devshm: true,
            mount_run: true,
            tmp_size_limit: 0,
            shm_size_limit: 0,
            run_size_limit: 0,
        }
    }
}

impl LinuxMountConfig {
    /// Minimal config: only /proc and /tmp (for containers).
    pub const fn minimal() -> Self {
        Self {
            mount_proc: true,
            mount_sys: false,
            mount_tmp: true,
            mount_devpts: false,
            mount_devshm: false,
            mount_run: false,
            tmp_size_limit: 64 * 1024 * 1024, // 64 MB
            shm_size_limit: 0,
            run_size_limit: 0,
        }
    }

    /// Full config for desktop/server (all enabled, generous limits).
    pub const fn full() -> Self {
        Self {
            mount_proc: true,
            mount_sys: true,
            mount_tmp: true,
            mount_devpts: true,
            mount_devshm: true,
            mount_run: true,
            tmp_size_limit: 0,
            shm_size_limit: 0,
            run_size_limit: 0,
        }
    }
}

// ── State tracking ──────────────────────────────────────────────────────────

static LINUX_VFS_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Result of mount setup — reports which mounts succeeded/failed.
#[derive(Debug, Clone)]
pub struct LinuxMountReport {
    pub entries: Vec<LinuxMountResult>,
}

#[derive(Debug, Clone)]
pub struct LinuxMountResult {
    pub mount_point: &'static str,
    pub fs_type: &'static str,
    pub success: bool,
    pub error: Option<&'static str>,
}

impl LinuxMountReport {
    fn new() -> Self {
        Self {
            entries: Vec::with_capacity(8),
        }
    }

    fn record(&mut self, mount_point: &'static str, fs_type: &'static str, result: Result<u64, &'static str>) {
        self.entries.push(LinuxMountResult {
            mount_point,
            fs_type,
            success: result.is_ok(),
            error: result.err(),
        });
    }

    /// Returns true if all attempted mounts succeeded.
    pub fn all_ok(&self) -> bool {
        self.entries.iter().all(|e| e.success)
    }

    /// Count of successful mounts.
    pub fn success_count(&self) -> usize {
        self.entries.iter().filter(|e| e.success).count()
    }

    /// Count of failed mounts.
    pub fn failure_count(&self) -> usize {
        self.entries.iter().filter(|e| !e.success).count()
    }
}

// ── Mount setup ─────────────────────────────────────────────────────────────

/// Initialize all Linux virtual filesystem mounts.
///
/// This function is idempotent — calling it multiple times is safe.
/// The mount table must be initialized before calling this.
///
/// # Arguments
/// * `config` — Controls which filesystems to mount and with what limits.
///
/// # Returns
/// A report detailing which mounts succeeded or failed.
pub fn setup_linux_vfs_mounts(config: &LinuxMountConfig) -> LinuxMountReport {
    let mut report = LinuxMountReport::new();

    // Prevent double initialization
    if LINUX_VFS_INITIALIZED.swap(true, Ordering::SeqCst) {
        return report;
    }

    // Ensure mount table is initialized
    mount_table::init_mount_table();

    // 1. /proc — Process information filesystem
    #[cfg(feature = "linux_compat")]
    if config.mount_proc {
        let result = mount_table::mount(
            "/proc",
            "proc",
            FsType::Procfs,
            MountFlags::NOSUID | MountFlags::NODEV | MountFlags::NOEXEC,
        );
        report.record("/proc", "procfs", result);

        if result.is_ok() {
            crate::klog_info!("linux_vfs: mounted /proc (procfs)");
        }
    }

    // 2. /sys — System information filesystem
    #[cfg(feature = "linux_compat")]
    if config.mount_sys {
        let result = mount_table::mount(
            "/sys",
            "sysfs",
            FsType::Sysfs,
            MountFlags::NOSUID | MountFlags::NODEV | MountFlags::NOEXEC,
        );
        report.record("/sys", "sysfs", result);

        if result.is_ok() {
            crate::klog_info!("linux_vfs: mounted /sys (sysfs)");
        }
    }

    // 3. /tmp — Temporary files (always available, not feature-gated)
    if config.mount_tmp {
        let result = mount_table::mount(
            "/tmp",
            "tmpfs",
            FsType::Tmpfs,
            MountFlags::NOSUID | MountFlags::NODEV,
        );
        report.record("/tmp", "tmpfs", result);

        if result.is_ok() {
            crate::klog_info!("linux_vfs: mounted /tmp (tmpfs)");
        }
    }

    // 4. /dev/pts — Pseudo-terminal directory
    #[cfg(feature = "linux_compat")]
    if config.mount_devpts {
        // Initialize PTY subsystem
        crate::modules::vfs::pty::init_pty_subsystem();

        let result = mount_table::mount(
            "/dev/pts",
            "devpts",
            FsType::Custom(1), // devpts type
            MountFlags::NOSUID | MountFlags::NOEXEC,
        );
        report.record("/dev/pts", "devpts", result);

        if result.is_ok() {
            crate::klog_info!("linux_vfs: mounted /dev/pts (devpts)");
        }
    }

    // 5. /dev/shm — POSIX shared memory
    if config.mount_devshm {
        let result = mount_table::mount(
            "/dev/shm",
            "tmpfs",
            FsType::Tmpfs,
            MountFlags::NOSUID | MountFlags::NODEV,
        );
        report.record("/dev/shm", "tmpfs", result);

        if result.is_ok() {
            crate::klog_info!("linux_vfs: mounted /dev/shm (tmpfs)");
        }
    }

    // 6. /run — Runtime data
    if config.mount_run {
        let result = mount_table::mount(
            "/run",
            "tmpfs",
            FsType::Tmpfs,
            MountFlags::NOSUID | MountFlags::NODEV,
        );
        report.record("/run", "tmpfs", result);

        if result.is_ok() {
            crate::klog_info!("linux_vfs: mounted /run (tmpfs)");
        }
    }

    // Seed the PRNG for /dev/random & /dev/urandom
    let seed = crate::hal::cpu::rdtsc();
    crate::modules::vfs::dev_special::seed_prng(seed);

    report
}

/// Tear down Linux VFS mounts (for namespace cleanup or shutdown).
pub fn teardown_linux_vfs_mounts() {
    if !LINUX_VFS_INITIALIZED.swap(false, Ordering::SeqCst) {
        return;
    }

    // Unmount in reverse order
    let mount_points = ["/run", "/dev/shm", "/dev/pts", "/tmp", "/sys", "/proc"];
    for mp in &mount_points {
        let _ = mount_table::unmount(mp);
    }
}

/// Check if Linux VFS mounts have been initialized.
pub fn is_linux_vfs_initialized() -> bool {
    LINUX_VFS_INITIALIZED.load(Ordering::Relaxed)
}
