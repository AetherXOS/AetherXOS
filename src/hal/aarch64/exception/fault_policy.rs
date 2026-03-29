use super::*;

#[inline(always)]
pub(super) fn is_lower_el_exception(frame: &ExceptionFrame) -> bool {
    (frame.spsr & 0b1111) == 0
}

fn terminate_current_task_and_halt(reason: &str, ec: u64, far: u64, elr: u64) -> ! {
    #[cfg(feature = "process_abstraction")]
    {
        if let Some(cpu) = unsafe { crate::kernel::cpu_local::CpuLocal::try_get() } {
            let tid = crate::interfaces::task::TaskId(cpu.current_task.load(Ordering::Relaxed));
            if tid.0 != 0 {
                let terminated = crate::kernel::launch::terminate_task(tid);
                if terminated {
                    crate::kernel::rt_preemption::request_forced_reschedule();
                }
                crate::klog_error!(
                    "AArch64 user exception terminated tid={} reason={} ec={:#x} far={:#x} elr={:#x} terminated={}",
                    tid.0,
                    reason,
                    ec,
                    far,
                    elr,
                    terminated
                );
            } else {
                crate::klog_error!(
                    "AArch64 user exception on idle task reason={} ec={:#x} far={:#x} elr={:#x}",
                    reason,
                    ec,
                    far,
                    elr
                );
            }
        } else {
            crate::klog_error!(
                "AArch64 user exception without CpuLocal reason={} ec={:#x} far={:#x} elr={:#x}",
                reason,
                ec,
                far,
                elr
            );
        }

        loop {
            crate::hal::HAL::halt();
        }
    }

    #[cfg(not(feature = "process_abstraction"))]
    {
        panic!(
            "AArch64 user exception without process_abstraction: reason={} ec={:#x} far={:#x} elr={:#x}",
            reason,
            ec,
            far,
            elr
        );
    }
}

#[inline(always)]
fn halt_current_core() -> ! {
    loop {
        crate::hal::HAL::halt();
    }
}

#[inline(always)]
fn user_fault_policy_allows_terminate(is_async: bool) -> bool {
    if is_async {
        AARCH64_EXCEPTION_KILL_USER_ASYNC
    } else {
        AARCH64_EXCEPTION_KILL_USER_SYNC
    }
}

pub(super) fn handle_user_fault(reason: &str, ec: u64, far: u64, elr: u64, is_async: bool) -> ! {
    if user_fault_policy_allows_terminate(is_async) {
        terminate_current_task_and_halt(reason, ec, far, elr);
    }
    panic!(
        "AArch64 user fault policy denied terminate: reason={} ec={:#x} far={:#x} elr={:#x} async={}",
        reason, ec, far, elr, is_async
    );
}

pub(super) fn handle_kernel_fault(
    reason: &str,
    ec: u64,
    far: u64,
    elr: u64,
    panic_enabled: bool,
) -> ! {
    if panic_enabled {
        panic!(
            "Kernel {}: ec={:#x} far={:#x} elr={:#x}",
            reason, ec, far, elr
        );
    }
    crate::klog_error!(
        "Kernel {} with panic disabled: ec={:#x} far={:#x} elr={:#x}; halting core",
        reason,
        ec,
        far,
        elr
    );
    halt_current_core()
}

#[no_mangle]
pub extern "C" fn unhandled_exception() {
    panic!("Unhandled AArch64 Exception!");
}
