use super::types::SwitchInfo;
use core::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use aethercore::interfaces::{Scheduler, SchedulerAction};

static SCHED_TRACE_EVENT_SEQ: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
fn should_emit_scheduler_trace() -> bool {
    let seq = SCHED_TRACE_EVENT_SEQ
        .fetch_add(1, AtomicOrdering::Relaxed)
        .saturating_add(1);
    aethercore::config::KernelConfig::should_emit_scheduler_trace_sample(seq)
}

#[inline(always)]
fn should_emit_timer_scheduler_serial(
    runqueue_len: usize,
    action: &SchedulerAction,
    force_rt_reschedule: bool,
) -> bool {
    runqueue_len != 0 || *action == SchedulerAction::Reschedule || force_rt_reschedule
}

pub(super) fn prepare_scheduler_switch(
    cpu: &'static aethercore::kernel::cpu_local::CpuLocal,
    current_tid: aethercore::interfaces::task::TaskId,
) -> Option<SwitchInfo> {
    use core::sync::atomic::Ordering;

    let mut scheduler = cpu.scheduler.lock();
    let action = scheduler.tick(current_tid);
    let idle_has_runnable_work = current_tid.0 == 0 && scheduler.runqueue_len() != 0;
    let force_rt_reschedule =
        aethercore::kernel::rt_preemption::on_scheduler_tick(&action, scheduler.runqueue_len());
    let runqueue_len = scheduler.runqueue_len();

    aethercore::kernel::debug_trace::record_optional(
        "timer.tick",
        "scheduler_tick_evaluated",
        Some(runqueue_len as u64),
        false,
    );

    #[cfg(target_arch = "x86_64")]
    if should_emit_timer_scheduler_serial(runqueue_len, &action, force_rt_reschedule) {
        aethercore::hal::serial::write_raw(
            "[EARLY SERIAL] timer scheduler tick evaluated\n",
        );
    }

    if action != aethercore::interfaces::SchedulerAction::Reschedule
        && !force_rt_reschedule
        && !idle_has_runnable_work
    {
        #[cfg(target_arch = "x86_64")]
        if should_emit_timer_scheduler_serial(runqueue_len, &action, force_rt_reschedule) {
            aethercore::hal::serial::write_raw(
                "[EARLY SERIAL] timer scheduler no switch requested\n",
            );
        }
        return None;
    }

    #[cfg(target_arch = "x86_64")]
    aethercore::hal::serial::write_raw("[EARLY SERIAL] timer pick_next call begin\n");
    #[cfg(feature = "sched_cfs")]
    let bootstrap_tid = if current_tid.0 == 0 && scheduler.runqueue_len() == 1 {
        #[cfg(target_arch = "x86_64")]
        aethercore::hal::serial::write_raw(
            "[EARLY SERIAL] timer bootstrap singleton path\n",
        );
        scheduler.bootstrap_pick_next()
    } else {
        None
    };
    #[cfg(not(feature = "sched_cfs"))]
    let bootstrap_tid: Option<aethercore::interfaces::task::TaskId> = None;
    let next_tid = if let Some(tid) = bootstrap_tid {
        #[cfg(target_arch = "x86_64")]
        aethercore::hal::serial::write_raw(
            "[EARLY SERIAL] timer bootstrap singleton returned\n",
        );
        tid
    } else if let Some(tid) = scheduler.pick_next() {
        #[cfg(target_arch = "x86_64")]
        aethercore::hal::serial::write_raw("[EARLY SERIAL] timer pick_next returned\n");
        tid
    } else {
        #[cfg(target_arch = "x86_64")]
        aethercore::hal::serial::write_raw("[EARLY SERIAL] timer pick_next empty\n");
        let stolen_task = aethercore::kernel::load_balance::try_steal_for_cpu(cpu)?;
        let tid = stolen_task.lock().id;
        scheduler.add_task(stolen_task);
        if should_emit_scheduler_trace() {
            aethercore::klog_trace!("Scheduler stole task {} onto CPU {}", tid, cpu.cpu_id.0);
        }
        tid
    };

    if next_tid == current_tid {
        #[cfg(target_arch = "x86_64")]
        aethercore::hal::serial::write_raw("[EARLY SERIAL] timer next equals current\n");
        return None;
    }

    let next_sp = scheduler
        .get_task_mut(next_tid)
        .map(|task| task.lock().kernel_stack_pointer as usize);
    let current_sp_ptr = if current_tid.0 == 0 {
        Some(cpu.idle_stack_pointer.as_ptr())
    } else {
        scheduler.get_task_mut(current_tid).map(|task| {
            let raw =
                alloc::sync::Arc::as_ptr(task) as *mut aethercore::interfaces::task::KernelTask;
            unsafe { &raw mut (*raw).kernel_stack_pointer as *mut u64 as *mut usize }
        })
    };

    #[cfg(all(feature = "ring_protection", target_arch = "x86_64"))]
    let (next_kernel_sp, next_tls, next_cr3) = scheduler
        .get_task_mut(next_tid)
        .map(|task| {
            let locked = task.lock();
            (
                locked.kernel_stack_pointer as usize,
                locked.user_tls_base,
                locked.page_table_root,
            )
        })
        .unwrap_or((0, 0, 0));
    #[cfg(not(all(feature = "ring_protection", target_arch = "x86_64")))]
    let (next_kernel_sp, next_tls, next_cr3) = (0usize, 0u64, 0u64);

    if current_tid.0 == 0 {
        if let Some(task_arc) = scheduler.get_task_mut(next_tid) {
            task_arc.lock().state = aethercore::interfaces::task::TaskState::Running;
        }
    }

    match (next_sp, current_sp_ptr) {
        (Some(next_sp), Some(current_sp_ptr)) => {
            #[cfg(target_arch = "x86_64")]
            aethercore::hal::serial::write_raw(
                "[EARLY SERIAL] timer scheduler switch ready\n",
            );
            if should_emit_scheduler_trace() {
                aethercore::klog_trace!(
                    "Scheduler switch cpu={} {} -> {}",
                    cpu.cpu_id.0,
                    current_tid,
                    next_tid
                );
            }
            cpu.current_task.store(next_tid.0, Ordering::Relaxed);
            Some(SwitchInfo {
                next_sp,
                current_sp_ptr,
                #[cfg(all(feature = "ring_protection", target_arch = "x86_64"))]
                next_tls,
                #[cfg(all(feature = "ring_protection", target_arch = "x86_64"))]
                next_cr3,
                #[cfg(all(feature = "ring_protection", target_arch = "x86_64"))]
                next_kernel_sp,
            })
        }
        (None, Some(_)) => {
            #[cfg(target_arch = "x86_64")]
            aethercore::hal::serial::write_raw("[EARLY SERIAL] timer next_sp missing\n");
            None
        }
        (Some(_), None) => {
            #[cfg(target_arch = "x86_64")]
            aethercore::hal::serial::write_raw(
                "[EARLY SERIAL] timer current_sp_ptr missing\n",
            );
            None
        }
        (None, None) => {
            #[cfg(target_arch = "x86_64")]
            aethercore::hal::serial::write_raw(
                "[EARLY SERIAL] timer both_sp_paths missing\n",
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::should_emit_timer_scheduler_serial;
    use aethercore::interfaces::SchedulerAction;

    #[test_case]
    fn timer_scheduler_serial_gate_stays_quiet_when_idle_and_empty() {
        assert!(!should_emit_timer_scheduler_serial(
            0,
            &SchedulerAction::Continue,
            false
        ));
    }

    #[test_case]
    fn timer_scheduler_serial_gate_logs_when_scheduler_work_exists() {
        assert!(should_emit_timer_scheduler_serial(
            1,
            &SchedulerAction::Continue,
            false
        ));
        assert!(should_emit_timer_scheduler_serial(
            0,
            &SchedulerAction::Reschedule,
            false
        ));
        assert!(should_emit_timer_scheduler_serial(
            0,
            &SchedulerAction::Continue,
            true
        ));
    }
}
