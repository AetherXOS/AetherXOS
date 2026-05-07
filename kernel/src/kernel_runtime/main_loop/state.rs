use core::sync::atomic::{AtomicBool, AtomicUsize, AtomicU64};

pub static INITRD_MOUNTED: AtomicBool = AtomicBool::new(false);
pub static LINUX_COMPAT_INITED: AtomicBool = AtomicBool::new(false);
pub static MAIN_LOOP_ITERATIONS: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "process_abstraction")]
pub static LINKED_PROBE_PID: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "process_abstraction")]
pub static LINKED_PROBE_SPAWNED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "process_abstraction")]
pub static LINKED_PROBE_VERIFIED: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "process_abstraction")]
pub static LINKED_PROBE_ENABLED: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "vfs")]
pub static VFS_SLO_SAMPLE_COUNTER: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub static VFS_SLO_BREACH_STREAK: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub static VFS_SLO_POLICY_ACTIONS: AtomicU64 = AtomicU64::new(0);
#[cfg(feature = "vfs")]
pub static VFS_SLO_LAST_LOG_SAMPLE: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "vfs")]
pub const VFS_SLO_SAMPLE_INTERVAL: u64 = 512;
#[cfg(feature = "vfs")]
pub const VFS_SLO_LOG_INTERVAL_MULTIPLIER: u64 = 8;
#[cfg(feature = "vfs")]
pub const VFS_SLO_ACTION_STREAK_THRESHOLD: u64 = 2;

#[cfg(all(feature = "vfs", feature = "linux_compat"))]
pub static COMPAT_SURFACE_SAMPLE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainLoopOneShotAction {
    Skip,
    Attempt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MainLoopIterationDecision {
    pub initrd_mount: MainLoopOneShotAction,
    pub linux_compat_init: MainLoopOneShotAction,
    #[cfg(feature = "process_abstraction")]
    pub linked_probe: super::probe::LinkedProbeMainLoopAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MainLoopIterationState {
    pub initrd_mounted: bool,
    pub linux_compat_inited: bool,
    #[cfg(feature = "process_abstraction")]
    pub linked_probe_enabled: bool,
    #[cfg(feature = "process_abstraction")]
    pub linked_probe_verified: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MainLoopBootState {
    pub boot_info_present: bool,
    #[cfg(feature = "process_abstraction")]
    pub linked_probe_enabled: bool,
}
