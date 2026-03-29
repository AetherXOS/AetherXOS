use super::check;
use super::BootHealthReport;
use crate::config::KernelConfig;
use crate::generated_consts::KERNEL_MAX_CPUS;
#[cfg(feature = "sched_lottery")]
use crate::generated_consts::SCHED_LOTTERY_REPLAY_TRACE_CAPACITY;

pub(super) fn run_core_checks(report: &mut BootHealthReport) {
    check(
        report,
        1001,
        KernelConfig::time_slice() > 0,
        "time_slice_ns must be > 0",
    );
    check(
        report,
        1002,
        KernelConfig::stack_size() > 0,
        "stack_size must be > 0",
    );
    check(
        report,
        1003,
        KernelConfig::watchdog_hard_stall_ns() >= KernelConfig::time_slice(),
        "watchdog_hard_stall_ns must be >= time_slice_ns",
    );
    check(
        report,
        1004,
        !KernelConfig::is_soft_watchdog_enabled() || KernelConfig::soft_watchdog_stall_ticks() > 0,
        "soft watchdog enabled but stall ticks is 0",
    );
    check(
        report,
        1005,
        KernelConfig::irq_vector_base() >= 32 && KernelConfig::irq_vector_base() <= 240,
        "irq_vector_base out of [32,240]",
    );
    check(
        report,
        1006,
        KernelConfig::launch_max_process_name_len() > 0
            && KernelConfig::launch_max_process_name_len() <= 32,
        "launch_max_process_name_len out of bounds",
    );
    check(
        report,
        1007,
        KernelConfig::launch_max_boot_image_bytes() > 0,
        "launch_max_boot_image_bytes must be > 0",
    );
    check(
        report,
        1008,
        KernelConfig::vfs_max_mount_path() > 0 && KernelConfig::vfs_max_mount_path() <= 4096,
        "vfs_max_mount_path out of bounds",
    );
    check(
        report,
        1009,
        KERNEL_MAX_CPUS > 0,
        "kernel max cpus must be > 0",
    );
    check(
        report,
        1010,
        KernelConfig::rebalance_prefer_local_skip_budget()
            <= KernelConfig::rebalance_batch_size().saturating_mul(16),
        "rebalance_prefer_local_skip_budget is unreasonably high for batch size",
    );
    #[cfg(feature = "sched_lottery")]
    check(
        report,
        1301,
        SCHED_LOTTERY_REPLAY_TRACE_CAPACITY > 0,
        "lottery replay trace capacity must be > 0",
    );
}
