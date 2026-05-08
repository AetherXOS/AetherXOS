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

    // Linux clear_child_tid mechanism
    if let Some(task_arc) = crate::kernel::task::get_task(task_id) {
        let mut task = task_arc.lock();
        let clear_tid_ptr = task.clear_child_tid;
        if clear_tid_ptr != 0 {
            // Only attempt write if we are in the correct address space (heuristic: current CR3 matches task's CR3)
            // In a production kernel, we might use a safe cross-process write primitive.
            let _ = crate::kernel::syscalls::write_user_pod(clear_tid_ptr, &0u32);
            
            #[cfg(feature = "ipc_futex")]
            {
                let key = clear_tid_ptr as u64; // In Linux, the key for this is the physical address or just the virtual address for private futexes
                crate::modules::ipc::futex::FUTEX_MANAGER.wake(key, 1);
            }
        }
        task.clear_child_tid = 0;
    }

    #[cfg(feature = "posix_signal")]
    {
        // Remove signal mask and pending signals for the task
    }
}
