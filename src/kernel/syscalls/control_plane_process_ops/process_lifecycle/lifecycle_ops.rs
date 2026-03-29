use super::super::*;

pub(crate) fn sys_spawn_process(
    image_ptr: usize,
    image_len: usize,
    name_ptr: usize,
    name_len: usize,
    priority: usize,
    deadline: usize,
) -> usize {
    SYSCALL_PROCESS_SPAWN_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_PROCESS_SPAWN)
    {
        return err;
    }

    let Some(priority_u8) = parse_process_priority(priority) else {
        return invalid_arg();
    };

    #[cfg(feature = "process_abstraction")]
    {
        let image_limit = crate::config::KernelConfig::launch_max_boot_image_bytes();
        let name_limit = crate::config::KernelConfig::launch_max_process_name_len();
        with_user_read_bounded_bytes(image_ptr, image_len, image_limit, |image| {
            with_user_read_bounded_bytes(name_ptr, name_len, name_limit, |name| {
                #[cfg(target_arch = "x86_64")]
                #[cfg(feature = "ring_protection")]
                let kernel_stack_top = crate::hal::x86_64::smp::allocate_kernel_stack_top() as u64;
                #[cfg(not(feature = "ring_protection"))]
                let kernel_stack_top = 0;
                #[cfg(not(target_arch = "x86_64"))]
                let kernel_stack_top = 0u64;
                let spawn = crate::kernel::launch::spawn_bootstrap_from_image(
                    name,
                    image,
                    priority_u8,
                    deadline as u64,
                    0,
                    kernel_stack_top,
                );

                match spawn {
                    Ok((process_id, _task_id)) => process_id,
                    Err(_) => invalid_arg(),
                }
            })
            .unwrap_or_else(|err| err)
        })
        .unwrap_or_else(|err| err)
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_terminate_process(process_id: usize) -> usize {
    SYSCALL_PROCESS_TERMINATE_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_PROCESS_KILL)
    {
        return err;
    }

    #[cfg(feature = "process_abstraction")]
    {
        let pid = crate::interfaces::task::ProcessId(process_id);
        if crate::kernel::launch::terminate_process(pid) {
            0
        } else {
            invalid_arg()
        }
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_terminate_task(task_id: usize) -> usize {
    SYSCALL_TASK_TERMINATE_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_PROCESS_KILL)
    {
        return err;
    }

    #[cfg(feature = "process_abstraction")]
    {
        let tid = crate::interfaces::task::TaskId(task_id);
        if crate::kernel::launch::terminate_task(tid) {
            0
        } else {
            invalid_arg()
        }
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_resolve_plt(slot_vaddr: usize, _name_ptr: usize) -> usize {
    let _ = (slot_vaddr, _name_ptr);
    !0
}
