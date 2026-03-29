use super::*;

pub(crate) fn sys_yield() -> usize {
    crate::kernel::rt_preemption::request_forced_reschedule();
    0
}

pub(crate) fn sys_exit(_code: usize) -> usize {
    crate::klog_info!("SYSCALL task exit code {}", _code);

    #[cfg(feature = "process_abstraction")]
    {
        let current_tid = unsafe {
            crate::kernel::cpu_local::CpuLocal::get()
                .current_task
                .load(core::sync::atomic::Ordering::Relaxed)
        };
        let current_tid = crate::interfaces::task::TaskId(current_tid);

        if let Some(pid) = crate::kernel::launch::process_id_by_task(current_tid) {
            if crate::kernel::launch::terminate_process_with_status(pid, _code as i32) {
                crate::kernel::rt_preemption::request_forced_reschedule();
            } else {
                crate::klog_warn!(
                    "SYSCALL exit: failed to terminate process pid={} status={}",
                    pid.0,
                    _code
                );
            }
        } else if crate::kernel::launch::terminate_task(current_tid) {
            crate::kernel::rt_preemption::request_forced_reschedule();
        } else {
            crate::klog_warn!(
                "SYSCALL exit: no process/task registered for tid={} status={}",
                current_tid.0,
                _code
            );
        }
    }

    use crate::interfaces::HardwareAbstraction;
    loop {
        crate::hal::HAL::halt();
    }
}

pub(super) fn sys_print(ptr: usize, len: usize) -> usize {
    SYSCALL_PRINT_CALLS.fetch_add(1, Ordering::Relaxed);

    with_user_read_bounded_bytes(ptr, len, MAX_PRINT_LEN, |slice| {
        if let Ok(s) = core::str::from_utf8(slice) {
            crate::klog_info!("USER: {}", s);
            len
        } else {
            invalid_arg()
        }
    })
    .unwrap_or_else(|err| err)
}

pub(crate) fn sys_set_tls(base: usize) -> usize {
    SYSCALL_TLS_CALLS.fetch_add(1, Ordering::Relaxed);

    if !crate::generated_consts::CORE_ENABLE_TLS_SYSCALLS {
        return invalid_arg();
    }

    if base != 0 && base >= USER_SPACE_TOP_EXCLUSIVE {
        return invalid_arg();
    }

    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);

    let mut scheduler = cpu.scheduler.lock();
    let Some(task_arc) = scheduler.get_task_mut(crate::interfaces::task::TaskId(current_tid))
    else {
        return invalid_arg();
    };
    #[cfg(feature = "ring_protection")]
    {
        task_arc.lock().user_tls_base = base as u64;
    }

    #[cfg(target_arch = "x86_64")]
    {
        crate::hal::cpu::ArchCpuRegisters::write_tls_base(base as u64);
    }

    0
}

pub(crate) fn sys_get_tls() -> usize {
    SYSCALL_TLS_CALLS.fetch_add(1, Ordering::Relaxed);

    if !crate::generated_consts::CORE_ENABLE_TLS_SYSCALLS {
        return invalid_arg();
    }

    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);

    let mut scheduler = cpu.scheduler.lock();
    if let Some(task_arc) = scheduler.get_task_mut(crate::interfaces::task::TaskId(current_tid)) {
        #[cfg(feature = "ring_protection")]
        {
            return task_arc.lock().user_tls_base as usize;
        }
    }

    #[cfg(target_arch = "x86_64")]
    {
        return crate::hal::cpu::ArchCpuRegisters::read_tls_base() as usize;
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        0
    }
}

pub(super) fn sys_set_affinity(mask: usize) -> usize {
    SYSCALL_AFFINITY_CALLS.fetch_add(1, Ordering::Relaxed);

    if !crate::generated_consts::CORE_ENABLE_AFFINITY_ENFORCEMENT {
        return invalid_arg();
    }

    let affinity_mask = mask as u64;
    if affinity_mask == 0 {
        return invalid_arg();
    }

    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);

    let mut scheduler = cpu.scheduler.lock();
    let Some(task_arc) = scheduler.get_task_mut(crate::interfaces::task::TaskId(current_tid))
    else {
        return invalid_arg();
    };
    let mut task = task_arc.lock();

    task.cpu_affinity_mask = affinity_mask;

    if !task.can_run_on_cpu_id(cpu.cpu_id) {
        if crate::generated_consts::CORE_ENABLE_SCHEDULER_TRACE {
            crate::klog_trace!(
                "affinity update requests migration task={} cpu={} mask={:#x}",
                current_tid,
                cpu.cpu_id,
                affinity_mask
            );
        }
        return SYSCALL_AFFINITY_MIGRATE_REQUIRED;
    }

    0
}

pub(super) fn sys_get_affinity() -> usize {
    SYSCALL_AFFINITY_CALLS.fetch_add(1, Ordering::Relaxed);

    if !crate::generated_consts::CORE_ENABLE_AFFINITY_ENFORCEMENT {
        return invalid_arg();
    }

    let cpu = unsafe { crate::kernel::cpu_local::CpuLocal::get() };
    let current_tid = cpu.current_task.load(core::sync::atomic::Ordering::Relaxed);

    let mut scheduler = cpu.scheduler.lock();
    let Some(task_arc) = scheduler.get_task_mut(crate::interfaces::task::TaskId(current_tid))
    else {
        return invalid_arg();
    };
    let mask = task_arc.lock().cpu_affinity_mask as usize;
    mask
}

pub(super) fn sys_get_abi_info(ptr: usize, len: usize) -> usize {
    SYSCALL_ABI_INFO_CALLS.fetch_add(1, Ordering::Relaxed);

    with_user_write_words(ptr, len, SYSCALL_ABI_INFO_WORDS, |out| {
        out.copy_from_slice(&[
            SYSCALL_ABI_MAGIC,
            SYSCALL_ABI_VERSION_MAJOR,
            SYSCALL_ABI_VERSION_MINOR,
            SYSCALL_ABI_VERSION_PATCH,
            SYSCALL_ABI_MIN_COMPAT_MAJOR,
            nr::GET_ABI_INFO,
            SYSCALL_ABI_FLAG_STABLE,
        ]);
        required_bytes(SYSCALL_ABI_INFO_WORDS)
    })
    .unwrap_or_else(|err| err)
}

pub(super) fn sys_get_crash_report(ptr: usize, len: usize) -> usize {
    SYSCALL_CRASH_REPORT_CALLS.fetch_add(1, Ordering::Relaxed);
    let crash = crate::kernel::crash_report();
    with_user_write_words(ptr, len, CRASH_REPORT_WORDS, |out| {
        out[0] = crash.panic_count as usize;
        out[1] = crash.last_panic_tick as usize;
        out[2] = crash.last_reason_hash as usize;
        out[3] = crash.watchdog_tick as usize;
        out[4] = crash.watchdog_stalls as usize;
        out[5] = crash.watchdog_hard_panics as usize;
        out[6] = crash.startup_stage_transitions as usize;
        out[7] = crash.startup_order_violations as usize;
        out[8] = crash.crash_log_latest_seq as usize;
        out[9] = crash.crash_log_latest_kind as usize;
        required_bytes(CRASH_REPORT_WORDS)
    })
    .unwrap_or_else(|err| err)
}

pub(super) fn sys_list_crash_events(ptr: usize, len: usize) -> usize {
    SYSCALL_CRASH_EVENTS_CALLS.fetch_add(1, Ordering::Relaxed);

    let bytes_per_event = required_bytes(CRASH_EVENT_WORDS);
    let capacity_events = len / bytes_per_event;
    if capacity_events == 0 {
        return invalid_arg();
    }

    let available = crate::kernel::crash_log::event_count();
    if available == 0 {
        return 0;
    }

    let event_count = capacity_events.min(available);
    let words_len = event_count.saturating_mul(CRASH_EVENT_WORDS);
    with_user_write_words_exact(ptr, len, words_len, |out| {
        let mut temp = [crate::kernel::crash_log::CrashEvent::EMPTY;
            crate::generated_consts::CORE_CRASH_LOG_CAPACITY];
        let written = crate::kernel::crash_log::recent_into(&mut temp[..event_count]);
        let mut cursor = 0usize;
        for ev in temp.iter().take(written) {
            out[cursor] = ev.seq as usize;
            out[cursor + 1] = ev.kind as usize;
            out[cursor + 2] = ev.tick as usize;
            out[cursor + 3] = ev.cpu_id as usize;
            out[cursor + 4] = ev.task_id as usize;
            out[cursor + 5] = ev.reason_hash as usize;
            out[cursor + 6] = ev.aux0 as usize;
            out[cursor + 7] = ev.aux1 as usize;
            cursor += CRASH_EVENT_WORDS;
        }
        required_bytes(written.saturating_mul(CRASH_EVENT_WORDS))
    })
    .unwrap_or_else(|err| err)
}

pub(super) fn sys_get_core_pressure_snapshot(ptr: usize, len: usize) -> usize {
    SYSCALL_CORE_PRESSURE_SNAPSHOT_CALLS.fetch_add(1, Ordering::Relaxed);
    let pressure = crate::kernel::pressure::snapshot();

    with_user_write_words(ptr, len, CORE_PRESSURE_SNAPSHOT_WORDS, |out| {
        write_core_pressure_snapshot_words(out, pressure);
        required_bytes(CORE_PRESSURE_SNAPSHOT_WORDS)
    })
    .unwrap_or_else(|err| err)
}
