mod aarch64;
mod dispatcher;
mod runtime;
mod virtualization_support;
mod x86_64;

pub(crate) use virtualization_support::{
    current_virtualization_log_snapshot, current_virtualization_policy_log_snapshot,
};

pub(crate) use self::runtime::{
    log_allocator_diagnostics, log_launch_pipeline, log_load_balance_runtime,
    log_module_loader_runtime, log_power_baseline, log_rt_preemption_guard, log_serial_runtime,
    log_syscall_runtime, log_watchdog_runtime,
};

#[cfg(feature = "allocators")]
pub(crate) use self::runtime::log_slab_runtime;

#[cfg(feature = "dispatcher")]
pub(crate) use self::dispatcher::{log_dispatcher_upcall_runtime, log_dispatcher_vectored_runtime};

#[cfg(target_arch = "aarch64")]
pub(crate) use self::aarch64::log_aarch64_exception_runtime;

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
pub(crate) use self::x86_64::log_x86_irq_runtime;
