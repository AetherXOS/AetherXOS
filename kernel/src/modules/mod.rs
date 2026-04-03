#[cfg(feature = "allocators")]
pub mod allocators;
#[cfg(feature = "dispatcher")]
pub mod dispatcher;
#[cfg(feature = "drivers")]
pub mod drivers;
#[cfg(feature = "governor")]
pub mod governor;
#[cfg(feature = "ipc")]
pub mod ipc;
#[cfg(feature = "networking")]
pub mod libnet;
#[cfg(feature = "linux_compat")]
pub mod linux_compat;
#[cfg(feature = "networking")]
pub mod network;
#[cfg(any(
    feature = "networking",
    feature = "vfs",
    feature = "process_abstraction"
))]
pub mod posix;
pub mod posix_consts;
#[cfg(feature = "schedulers")]
pub mod schedulers;
#[cfg(feature = "security")]
pub mod security;
#[cfg(feature = "vfs")]
pub mod vfs;
#[cfg(feature = "linux_userspace_graphics")]
pub mod userspace_graphics;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibrarySurfacePolicy {
    CoreOnly,
    CorePlusSelectedLibraries,
    CompatAll,
}

pub fn library_surface_policy() -> LibrarySurfacePolicy {
    match crate::config::KernelConfig::boundary_mode() {
        crate::config::BoundaryMode::Strict => LibrarySurfacePolicy::CoreOnly,
        crate::config::BoundaryMode::Balanced => LibrarySurfacePolicy::CorePlusSelectedLibraries,
        crate::config::BoundaryMode::Compat => LibrarySurfacePolicy::CompatAll,
    }
}

pub fn is_library_surface_enabled(name: &str) -> bool {
    match name {
        "vfs" => crate::config::KernelConfig::is_vfs_library_api_exposed(),
        "network" => crate::config::KernelConfig::is_network_library_api_exposed(),
        "ipc" => crate::config::KernelConfig::is_ipc_library_api_exposed(),
        "proc_config" => crate::config::KernelConfig::should_expose_procfs_surface(),
        "sysctl" => crate::config::KernelConfig::should_expose_sysctl_surface(),
        "linux_compat_surface" => crate::config::KernelConfig::should_expose_linux_compat_surface(),
        _ => false,
    }
}

pub mod selector {
    #[cfg(feature = "schedulers")]
    pub use crate::modules::schedulers::selector::ActiveScheduler;

    #[cfg(not(feature = "schedulers"))]
    pub type ActiveScheduler = NoopScheduler;

    #[cfg(feature = "schedulers")]
    use crate::kernel::sync::IrqSafeMutex;

    #[cfg(feature = "schedulers")]
    pub static GLOBAL_SCHEDULER: IrqSafeMutex<Option<ActiveScheduler>> = IrqSafeMutex::new(None);

    #[cfg(feature = "schedulers")]
    #[inline(never)]
    pub fn bootstrap_active_scheduler() -> ActiveScheduler {
        #[cfg(target_arch = "x86_64")]
        crate::hal::serial::write_raw(
            "[EARLY SERIAL] bootstrap active scheduler wrapper begin\n",
        );
        let scheduler = ActiveScheduler::new();
        #[cfg(target_arch = "x86_64")]
        crate::hal::serial::write_raw(
            "[EARLY SERIAL] bootstrap active scheduler wrapper returned\n",
        );
        scheduler
    }

    /// Fallback no-op scheduler for configurations without any scheduler module.
    /// In this mode, the kernel executes in a single-task cooperative loop.
    #[cfg(not(feature = "schedulers"))]
    pub struct NoopScheduler;

    #[cfg(not(feature = "schedulers"))]
    impl NoopScheduler {
        pub const fn new() -> Self {
            Self
        }
    }

    #[cfg(not(feature = "schedulers"))]
    #[inline(never)]
    pub fn bootstrap_active_scheduler() -> ActiveScheduler {
        #[cfg(target_arch = "x86_64")]
        crate::hal::serial::write_raw(
            "[EARLY SERIAL] bootstrap active scheduler wrapper begin\n",
        );
        let scheduler = NoopScheduler::new();
        #[cfg(target_arch = "x86_64")]
        crate::hal::serial::write_raw(
            "[EARLY SERIAL] bootstrap active scheduler wrapper returned\n",
        );
        scheduler
    }

    #[cfg(not(feature = "schedulers"))]
    impl crate::interfaces::Scheduler for NoopScheduler {
        type TaskItem = crate::interfaces::KernelTask;

        fn init(&mut self) {}
        fn add_task(&mut self, _task: Self::TaskItem) {}
        fn remove_task(&mut self, _task_id: crate::interfaces::task::TaskId) {}
        fn pick_next(&mut self) -> Option<crate::interfaces::task::TaskId> {
            None
        }
        fn tick(
            &mut self,
            _current_task: crate::interfaces::task::TaskId,
        ) -> crate::interfaces::SchedulerAction {
            crate::interfaces::SchedulerAction::Continue
        }
    }
}
