/// Global configuration for Linux compatibility layer.
/// Driven by hyper_config.toml via build.rs generated constants.

pub struct LinuxCompatConfig;

impl LinuxCompatConfig {
    /// Maximum length of a path string from user space.
    pub const MAX_PATH_LEN: usize = crate::generated_consts::LINUX_MAX_PATH_LEN;

    /// Maximum number of IOV entries for readv/writev.
    pub const MAX_IOV_COUNT: usize = crate::generated_consts::LINUX_MAX_IOV_COUNT;

    /// Maximum size of user-provided sockaddr payload.
    pub const MAX_SOCKADDR_LEN: usize = crate::generated_consts::LINUX_MAX_SOCKADDR_LEN;

    /// Maximum length of an extended attribute name.
    pub const MAX_XATTR_NAME_LEN: usize = crate::generated_consts::LINUX_MAX_XATTR_NAME_LEN;

    /// Maximum size of an extended attribute value.
    pub const MAX_XATTR_VALUE_SIZE: usize = crate::generated_consts::LINUX_MAX_XATTR_VALUE_SIZE;

    /// Default pipe buffer size.
    pub const DEFAULT_PIPE_SIZE: usize = crate::generated_consts::LINUX_DEFAULT_PIPE_SIZE;

    /// Maximum mount path length.
    pub const MAX_MOUNT_PATH: usize = crate::generated_consts::LINUX_MAX_MOUNT_PATH;

    /// File descriptor base for detached mount contexts.
    pub const MOUNT_CTX_FD_BASE: i32 = crate::generated_consts::LINUX_MOUNT_CTX_FD_BASE;

    /// File descriptor base for mount handles.
    pub const MOUNT_FD_BASE: i32 = crate::generated_consts::LINUX_MOUNT_FD_BASE;

    /// Maximum waiter entries accepted by futex_waitv.
    pub const FUTEX_WAITV_MAX: usize = crate::generated_consts::LINUX_FUTEX_WAITV_MAX;

    /// ABI size for robust-list head in current compat profile.
    pub const ROBUST_LIST_HEAD_SIZE: usize = crate::generated_consts::LINUX_ROBUST_LIST_HEAD_SIZE;

    /// Default block size reported by stat/statx.
    pub const STAT_BLOCK_SIZE: u64 = crate::generated_consts::LINUX_STAT_BLOCK_SIZE;
    pub const STAT_BLKSIZE: u32 = crate::generated_consts::LINUX_STAT_BLKSIZE;

    /// Verbosity of syscall logging.
    pub const VERBOSE_LOGS: bool = crate::generated_consts::LINUX_VERBOSE_SYSCALL_LOGS;

    /// Whether to map all errors to standard Linux errno values.
    pub const USE_STANDARD_ERRNO: bool =
        crate::generated_consts::LINUX_ENABLE_STANDARD_ERROR_MAPPING;

    /// Whether to enable legacy/obsolete syscall support (multiplexers, old variants).
    pub const LEGACY_SUPPORT: bool = crate::generated_consts::LINUX_LEGACY_SUPPORT;
}

// ── Runtime-mutable state ─────────────────────────────────────────────────────

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// 64-bit PRNG seed, seeded from RDRAND at init time.
/// Used by getrandom / urandom syscall emulation.
static PRNG_SEED: AtomicU64 = AtomicU64::new(0xDEAD_BEEF_CAFE_BABE);
static PTRACE_COMPAT_ENABLED: AtomicBool = AtomicBool::new(true);
static SECCOMP_COMPAT_ENABLED: AtomicBool = AtomicBool::new(true);
static MMAN_SOFT_FALLBACK_ENABLED: AtomicBool = AtomicBool::new(cfg!(feature = "linux_shim_noop_mlock"));
static WAYLAND_COMPAT_ENABLED: AtomicBool = AtomicBool::new(true);
static X11_COMPAT_ENABLED: AtomicBool = AtomicBool::new(true);

/// Seed the linux-compat PRNG (called once from `linux_compat::init()`).
pub fn set_prng_seed(seed: u64) {
    PRNG_SEED.store(seed, Ordering::Relaxed);
}

/// Generate the next pseudo-random 64-bit value (xorshift64).
#[inline(always)]
pub fn prng_next() -> u64 {
    let mut v = PRNG_SEED.load(Ordering::Relaxed);
    v ^= v << 13;
    v ^= v >> 7;
    v ^= v << 17;
    PRNG_SEED.store(v, Ordering::Relaxed);
    v
}

/// Runtime toggle for ptrace compatibility behavior.
pub fn set_ptrace_compat_enabled(enabled: bool) {
    PTRACE_COMPAT_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Runtime toggle for seccomp compatibility behavior.
pub fn set_seccomp_compat_enabled(enabled: bool) {
    SECCOMP_COMPAT_ENABLED.store(enabled, Ordering::Relaxed);
}

#[inline(always)]
pub fn ptrace_compat_enabled() -> bool {
    PTRACE_COMPAT_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn seccomp_compat_enabled() -> bool {
    SECCOMP_COMPAT_ENABLED.load(Ordering::Relaxed)
}

/// Runtime toggle for soft mman compatibility fallback behavior when
/// `vfs + posix_mman` features are not available.
pub fn set_mman_soft_fallback_enabled(enabled: bool) {
    MMAN_SOFT_FALLBACK_ENABLED.store(enabled, Ordering::Relaxed);
}

#[inline(always)]
pub fn mman_soft_fallback_enabled() -> bool {
    MMAN_SOFT_FALLBACK_ENABLED.load(Ordering::Relaxed)
}

/// Runtime toggle for Wayland userspace compatibility surface.
pub fn set_wayland_compat_enabled(enabled: bool) {
    WAYLAND_COMPAT_ENABLED.store(enabled, Ordering::Relaxed);
    #[cfg(feature = "linux_userspace_graphics")]
    {
        crate::modules::userspace_graphics::set_wayland_runtime_enabled(enabled);
    }
}

#[inline(always)]
pub fn wayland_compat_enabled() -> bool {
    WAYLAND_COMPAT_ENABLED.load(Ordering::Relaxed)
}

/// Runtime toggle for X11 userspace compatibility surface.
pub fn set_x11_compat_enabled(enabled: bool) {
    X11_COMPAT_ENABLED.store(enabled, Ordering::Relaxed);
    #[cfg(feature = "linux_userspace_graphics")]
    {
        crate::modules::userspace_graphics::set_x11_runtime_enabled(enabled);
    }
}

#[inline(always)]
pub fn x11_compat_enabled() -> bool {
    X11_COMPAT_ENABLED.load(Ordering::Relaxed)
}
