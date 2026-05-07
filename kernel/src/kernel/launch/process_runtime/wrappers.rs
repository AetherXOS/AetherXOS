use super::*;

#[cfg(feature = "process_abstraction")]
pub fn process_launch_context(process_id: usize) -> Option<LaunchContext> {
    process_launch_context_typed(ProcessId(process_id))
}

#[cfg(feature = "process_abstraction")]
pub fn process_boot_image(process_id: usize) -> Option<Vec<u8>> {
    process_boot_image_typed(ProcessId(process_id))
}

#[cfg(feature = "process_abstraction")]
pub fn acknowledge_launch_context(process_id: usize, success: bool) -> bool {
    acknowledge_launch_context_typed(ProcessId(process_id), success)
}

#[cfg(feature = "process_abstraction")]
pub fn launch_context_stage(process_id: usize) -> Option<usize> {
    launch_context_stage_typed(ProcessId(process_id))
}

#[cfg(feature = "process_abstraction")]
pub fn terminate_process(process_id: ProcessId) -> bool {
    terminate_process_with_status(process_id, 0)
}

pub fn finalize_task_user_exit_state(task_id: TaskId) {
    // Robust list cleanup is critical for thread-mutex synchronization in Linux
    crate::kernel::syscalls::clear_robust_list_for_tid(task_id.0);

    #[cfg(feature = "posix_signal")]
    {
        // Remove signal mask and pending signals for the task
        // Note: In AetherXOS, signal state is often indexed by PID for process-wide signals,
        // but some state might be task-specific in the future.
        // For now, we ensure the robust list is cleared.
    }
}
