mod allocators;
mod loader;
mod power;
mod scheduler;
mod serial;
mod syscalls;

pub(crate) use self::loader::{log_launch_pipeline, log_module_loader_runtime};
pub(crate) use self::power::log_power_baseline;
pub(crate) use self::scheduler::{
    log_load_balance_runtime, log_rt_preemption_guard, log_watchdog_runtime,
};
pub(crate) use self::serial::log_serial_runtime;

#[cfg(feature = "allocators")]
pub(crate) use self::allocators::{log_allocator_diagnostics, log_slab_runtime};

#[cfg(feature = "ring_protection")]
pub(crate) use self::syscalls::log_syscall_runtime;
