use super::super::*;

pub(crate) fn sys_get_launch_stats(ptr: usize, len: usize) -> usize {
    SYSCALL_LAUNCH_STATS_CALLS.fetch_add(1, Ordering::Relaxed);

    let stats = crate::kernel::launch::stats();
    write_user_words(
        ptr,
        len,
        [
            stats.spawn_attempts as usize,
            stats.spawn_success as usize,
            stats.spawn_failures as usize,
            stats.enqueue_failures as usize,
            stats.registered_processes,
            stats.last_task_id.0,
            stats.terminate_attempts as usize,
            stats.terminate_success as usize,
            stats.terminate_failures as usize,
            stats.validation_failures as usize,
            stats.claim_attempts as usize,
            stats.claim_success as usize,
            stats.claim_failures as usize,
            stats.handoff_ack_attempts as usize,
            stats.handoff_ack_success as usize,
            stats.handoff_ack_failures as usize,
            stats.handoff_consume_attempts as usize,
            stats.handoff_consume_success as usize,
            stats.handoff_consume_failures as usize,
            stats.terminate_by_task_attempts as usize,
            stats.terminate_by_task_success as usize,
            stats.terminate_by_task_failures as usize,
            stats.handoff_execute_attempts as usize,
            stats.handoff_execute_success as usize,
            stats.handoff_execute_failures as usize,
        ],
    )
}

pub(crate) fn sys_get_process_count() -> usize {
    SYSCALL_PROCESS_COUNT_CALLS.fetch_add(1, Ordering::Relaxed);
    #[cfg(feature = "process_abstraction")]
    {
        crate::kernel::launch::process_count()
    }
    #[cfg(not(feature = "process_abstraction"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_list_process_ids(ptr: usize, len: usize) -> usize {
    SYSCALL_PROCESS_LIST_CALLS.fetch_add(1, Ordering::Relaxed);

    if len == 0 {
        return invalid_arg();
    }

    let capacity = len / core::mem::size_of::<usize>();
    if capacity == 0 {
        return invalid_arg();
    }

    #[cfg(feature = "process_abstraction")]
    {
        let max_items = capacity.min(PROCESS_LIST_LIMIT);
        let mut buf = [crate::interfaces::task::ProcessId(0); PROCESS_LIST_LIMIT];
        let count = crate::kernel::launch::process_ids_snapshot(&mut buf[..max_items]);
        with_user_write_words_exact(ptr, len, count, |out| {
            for i in 0..count {
                out[i] = buf[i].0;
            }
            count
        })
        .unwrap_or_else(|err| err)
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_get_process_image_state(process_id: usize, ptr: usize, len: usize) -> usize {
    SYSCALL_PROCESS_IMAGE_STATE_CALLS.fetch_add(1, Ordering::Relaxed);

    #[cfg(feature = "process_abstraction")]
    {
        let pid = crate::interfaces::task::ProcessId(process_id);
        let Some((entry, pages, segments)) = crate::kernel::launch::process_image_state(pid) else {
            return invalid_arg();
        };

        write_user_words(ptr, len, [entry, pages, segments])
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_get_process_mapping_state(process_id: usize, ptr: usize, len: usize) -> usize {
    SYSCALL_PROCESS_MAPPING_STATE_CALLS.fetch_add(1, Ordering::Relaxed);

    #[cfg(feature = "process_abstraction")]
    {
        let pid = crate::interfaces::task::ProcessId(process_id);
        let Some((regions, pages)) = crate::kernel::launch::process_mapping_state(pid) else {
            return invalid_arg();
        };

        write_user_words(ptr, len, [regions, pages])
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_get_process_id_by_task(task_id: usize) -> usize {
    SYSCALL_TASK_PROCESS_ID_CALLS.fetch_add(1, Ordering::Relaxed);

    #[cfg(feature = "process_abstraction")]
    {
        crate::kernel::launch::process_id_by_task(crate::interfaces::task::TaskId(task_id))
            .map(|pid| pid.0)
            .unwrap_or(usize::MAX)
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        invalid_arg()
    }
}
