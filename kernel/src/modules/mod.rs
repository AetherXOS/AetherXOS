#[cfg(feature = "allocators")]
pub mod allocators;
pub mod benchmarks;
#[cfg(feature = "dispatcher")]
pub mod dispatcher;
pub mod event_notification;
pub mod fault_tolerance;
pub mod formal_verification;
pub mod gpu_compute;
pub mod hardware_crypto;
#[cfg(feature = "drivers")]
pub mod drivers;
#[cfg(feature = "governor")]
pub mod governor;
pub mod hotplug_migration;
#[cfg(feature = "ipc")]
pub mod ipc;
pub mod logging;
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
pub mod persistent_memory;
pub mod posix_consts;
pub mod power_management;
#[cfg(feature = "schedulers")]
pub mod schedulers;
pub mod memory_safety;
pub mod resource_manager;
pub mod realtime_scheduler;
pub mod secure_boot;
#[cfg(feature = "security")]
pub mod security;
pub mod syscall_inline;
pub mod stability;
pub mod adaptive_tuning;
pub mod userspace_io;
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
    #[cfg(feature = "sched_cfs")]
    pub use crate::modules::schedulers::cfs::CFS as ActiveScheduler;

    #[cfg(not(feature = "sched_cfs"))]
    pub struct NoopScheduler;

    #[cfg(not(feature = "sched_cfs"))]
    pub type ActiveScheduler = NoopScheduler;

    pub use crate::interfaces::Scheduler;

    #[cfg(not(feature = "sched_cfs"))]
    impl NoopScheduler {
        pub const fn new() -> Self {
            Self {}
        }
    }

    #[cfg(not(feature = "sched_cfs"))]
    impl crate::interfaces::Scheduler for NoopScheduler {
        type TaskItem = alloc::sync::Arc<crate::kernel::sync::IrqSafeMutex<crate::interfaces::KernelTask>>;

        fn get_task_mut(&mut self, _task_id: crate::interfaces::task::TaskId) -> Option<&mut Self::TaskItem> {
            None
        }
        fn steal_task(&mut self) -> Option<Self::TaskItem> {
            None
        }
        fn runqueue_len(&self) -> usize {
            0
        }
        fn cpu_load(&self) -> usize {
            0
        }
        fn init(&mut self) {}
        fn add_task(&mut self, _task: Self::TaskItem) {}
        fn remove_task(&mut self, _task_id: crate::interfaces::task::TaskId) {}
        fn remove_task_item(&mut self, _task_id: crate::interfaces::task::TaskId) -> Option<Self::TaskItem> {
            None
        }
        fn pick_next(&mut self) -> Option<crate::interfaces::task::TaskId> {
            None
        }
        fn tick(&mut self, _current_task: crate::interfaces::task::TaskId) -> crate::interfaces::SchedulerAction {
            crate::interfaces::SchedulerAction::Continue
        }
    }

    #[cfg(not(feature = "schedulers"))]
    pub fn bootstrap_active_scheduler() -> NoopScheduler {
        NoopScheduler::new()
    }

    #[cfg(feature = "schedulers")]
    use crate::kernel::sync::IrqSafeMutex;

    #[cfg(feature = "schedulers")]
    pub static GLOBAL_SCHEDULER: IrqSafeMutex<Option<ActiveScheduler>> = IrqSafeMutex::new(None);

    #[cfg(feature = "schedulers")]
    pub fn bootstrap_active_scheduler() -> ActiveScheduler {
        crate::hal::serial::write_raw(
            "[EARLY SERIAL] bootstrap active scheduler wrapper begin\n",
        );
        let scheduler = ActiveScheduler::new();
        crate::hal::serial::write_raw(
            "[EARLY SERIAL] bootstrap active scheduler wrapper returned\n",
        );
        scheduler
    }
}
