use super::*;

pub(super) fn kill(pid: usize, signal: i32) -> Result<(), PosixErrno> {
    if signal == 0 {
        return if process_exists(pid) {
            Ok(())
        } else {
            Err(PosixErrno::NoEntry)
        };
    }

    let Some(sig) = PosixSignal::from_raw(signal) else {
        return Err(PosixErrno::Invalid);
    };

    match sig {
        PosixSignal::Term | PosixSignal::Kill => {
            #[cfg(feature = "process_abstraction")]
            {
                let status = encode_wait_signal_status(signal);
                if crate::kernel::launch::terminate_process_with_status(
                    crate::interfaces::task::ProcessId(pid),
                    status,
                ) {
                    record_exit_status(pid, status, PosixRusage::of_process(pid));
                    // Metadata preserved for waitpid reaping.
                    Ok(())
                }
 else {
                    Err(PosixErrno::NoEntry)
                }
            }
            #[cfg(not(feature = "process_abstraction"))]
            {
                if pid == getpid() && pid != 0 {
                    record_exit_status(pid, encode_wait_signal_status(signal), PosixRusage::current_self());
                    crate::kernel::rt_preemption::request_forced_reschedule();
                    Ok(())
                } else {
                    Err(PosixErrno::NoEntry)
                }
            }
        }
    }
}

pub(super) fn exit_with_status(code: u8) -> Result<(), PosixErrno> {
    let pid = getpid();
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }

    let status = encode_wait_exit_status(code);

    #[cfg(feature = "process_abstraction")]
    {
        if crate::kernel::launch::terminate_process_with_status(
            crate::interfaces::task::ProcessId(pid),
            status,
        ) {
            record_exit_status(pid, status, PosixRusage::current_self());
            // Metadata MUST NOT be cleared here. It must persist until reaped by waitpid.
            crate::kernel::rt_preemption::request_forced_reschedule();
            Ok(())
        }
 else {
            Err(PosixErrno::NoEntry)
        }
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        record_exit_status(pid, status, PosixRusage::current_self());
        crate::kernel::rt_preemption::request_forced_reschedule();
        Ok(())
    }
}

#[inline(always)]
pub(super) fn _exit(code: u8) -> Result<(), PosixErrno> {
    exit_with_status(code)
}

pub(super) fn fork() -> Result<usize, PosixErrno> {
    #[cfg(feature = "process_abstraction")]
    {
        let parent_pid = getpid();
        if parent_pid == 0 {
            return Err(PosixErrno::Invalid);
        }

        // Use the standardized kernel fork primitive
        match crate::kernel::fork::do_fork(
            crate::interfaces::task::ProcessId(parent_pid),
            0, // stack handled by syscall return path
            0, // entry handled by syscall return path
            0, // inherit priority
        ) {
            Ok((child_pid, _child_tid)) => {
                ensure_process_metadata(parent_pid);
                register_spawned_process(parent_pid, child_pid.0);
                Ok(child_pid.0)
            }
            Err(_) => Err(PosixErrno::Again),
        }
    }


    #[cfg(not(feature = "process_abstraction"))]
    {
        Err(PosixErrno::NoSys)
    }
}

pub(super) fn fork_from_image(
    process_name: &[u8],
    image: &[u8],
    priority: u8,
    deadline: u64,
    burst_time: u64,
    kernel_stack_top: u64,
) -> Result<usize, PosixErrno> {
    posix_spawn_from_image(
        process_name,
        image,
        priority,
        deadline,
        burst_time,
        kernel_stack_top,
    )
}

pub(super) fn getpriority(pid: usize) -> Result<i32, PosixErrno> {
    let target = normalize_target_pid(getpid(), pid);
    if target == 0 || !process_exists(target) {
        return Err(PosixErrno::NoEntry);
    }
    Ok(*NICE_VALUES.lock().get(&target).unwrap_or(&0))
}

pub(super) fn setpriority(pid: usize, prio: i32) -> Result<(), PosixErrno> {
    let target = normalize_target_pid(getpid(), pid);
    if target == 0 || !process_exists(target) {
        return Err(PosixErrno::NoEntry);
    }
    NICE_VALUES.lock().insert(target, clamp_nice(prio));
    Ok(())
}

pub(super) fn nice(increment: i32) -> Result<i32, PosixErrno> {
    let pid = getpid();
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }
    let current = getpriority(pid)?;
    let next = clamp_nice(current.saturating_add(increment));
    setpriority(pid, next)?;
    Ok(next)
}

pub(super) fn raise(signal: i32) -> Result<(), PosixErrno> {
    let pid = getpid();
    if pid == 0 {
        return Err(PosixErrno::Invalid);
    }
    kill(pid, signal)
}

pub(super) fn killpg(pgid: usize, signal: i32) -> Result<(), PosixErrno> {
    let me = getpid();
    if me == 0 {
        return Err(PosixErrno::Invalid);
    }

    let group = if pgid == 0 { getpgrp() } else { pgid };
    if group == 0 {
        return Err(PosixErrno::Invalid);
    }

    #[cfg(feature = "process_abstraction")]
    {
        let mut mgr = crate::kernel::tty::job_control::GLOBAL_PROCESS_GROUP_MANAGER.lock();
        let delivery = crate::kernel::signal::group_delivery::SignalGroupDelivery::new(&mut *mgr);
        let pgrp_id = crate::kernel::tty::job_control::ProcessGroupId(crate::interfaces::task::ProcessId(group));
        
        if let Ok(result) = delivery.deliver_to_group(pgrp_id, signal as u32, true, false) {
            if result.group_affected {
                return Ok(());
            }
        }
        
        // Fallback to manual loop if delivery didn't affect anyone or failed
        let mut ids = [crate::interfaces::task::ProcessId(0); 64];
        let written = crate::kernel::launch::process_ids_snapshot(&mut ids);
        let mut delivered = false;
        for pid in &ids[..written] {
            if getpgid_of(pid.0).ok() != Some(group) {
                continue;
            }
            if kill(pid.0, signal).is_ok() {
                delivered = true;
            }
        }
        if delivered {
            Ok(())
        } else {
            Err(PosixErrno::NoEntry)
        }
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        if group != getpgrp() {
            return Err(PosixErrno::NoEntry);
        }
        kill(me, signal)
    }
}
