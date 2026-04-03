#[cfg(target_os = "none")]
use crate::kernel::syscalls::syscalls_consts::x86;
#[cfg(target_os = "none")]
use x86_64::structures::idt::{InterruptStackFrame, PageFaultErrorCode};
#[cfg(target_os = "none")]
use crate::hal::common::exception::{record_exception_snapshot, ExceptionSnapshot};

#[cfg(target_os = "none")]
fn handle_common_exception(
    name: &str,
    vector: u8,
    stack_frame: InterruptStackFrame,
    error_code: Option<u64>,
    fault_addr: Option<u64>,
) {
    let bytes = unsafe {
        core::slice::from_raw_parts(
            (&stack_frame as *const InterruptStackFrame).cast::<u8>(),
            core::mem::size_of::<InterruptStackFrame>(),
        )
    };
    record_exception_snapshot(ExceptionSnapshot {
        trace_label: name,
        dump_label: name,
        frame_bytes: bytes,
        instruction_pointer: stack_frame.instruction_pointer.as_u64(),
        stack_pointer: stack_frame.stack_pointer.as_u64(),
        fault_or_code: fault_addr.unwrap_or(error_code.unwrap_or(0)),
        status_or_flags: stack_frame.cpu_flags,
    });

    if super::dispatch::try_dispatch_vector(vector) {
        return;
    }

    crate::klog_error!(
        "EXCEPTION: {} (vector {}) at {:#x} code: {:#x?} frame: {:#?}",
        name,
        vector,
        stack_frame.instruction_pointer.as_u64(),
        error_code,
        stack_frame
    );
    crate::kernel::fatal_halt(name);
}

#[cfg(target_os = "none")]
pub(super) extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    let ex = super::metadata::exception_descriptor(x86::EXCEPTION_BREAKPOINT);
    handle_common_exception(ex.label, ex.id, stack_frame, None, None);
}

#[cfg(target_os = "none")]
pub(super) extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    crate::klog_error!("FATAL EXCEPTION: DOUBLE FAULT {:#?}", stack_frame);
    let ex = super::metadata::exception_descriptor(x86::EXCEPTION_DOUBLE_FAULT);
    handle_common_exception(ex.label, ex.id, stack_frame, Some(error_code), None);
    unreachable!()
}

#[cfg(target_os = "none")]
pub(super) extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use crate::interfaces::cpu::CpuRegisters;
    let addr = crate::hal::cpu::X86CpuRegisters::read_page_fault_addr();
    let ex = super::metadata::exception_descriptor(x86::EXCEPTION_PAGE_FAULT);
    handle_common_exception(ex.label, ex.id, stack_frame, Some(error_code.bits()), Some(addr));
}

#[cfg(target_os = "none")]
pub(super) extern "x86-interrupt" fn gpf_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    let ex = super::metadata::exception_descriptor(x86::EXCEPTION_GPF);
    handle_common_exception(ex.label, ex.id, stack_frame, Some(error_code), None);
}

#[cfg(target_os = "none")]
pub(super) extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    super::dispatch::dispatch_irq_vector(x86::IRQ_TIMER);
}
