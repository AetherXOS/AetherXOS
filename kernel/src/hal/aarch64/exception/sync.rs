use super::*;
use super::fault_policy::{handle_kernel_fault, handle_user_fault, is_lower_el_exception};
#[path = "sync_diagnostics.rs"]
mod sync_diagnostics;

use crate::generated_consts::{
    AARCH64_EXCEPTION_PANIC_ON_KERNEL_ASYNC, AARCH64_EXCEPTION_PANIC_ON_KERNEL_SYNC,
};

const ESR_EC_SVC64: u64 = 0b010101;
const ESR_EC_IABORT_LOWER_EL: u64 = 0b100000;
const ESR_EC_IABORT_CURRENT_EL: u64 = 0b100001;
const ESR_EC_DABORT_LOWER_EL: u64 = 0b100100;
const ESR_EC_DABORT_CURRENT_EL: u64 = 0b100101;

const AARCH64_INSTRUCTION_WIDTH_BYTES: u64 = 4;
const ESR_EC_SHIFT: u64 = 26;
const ESR_EC_MASK: u64 = 0x3F;
const FAR_NOT_AVAILABLE: u64 = 0;

#[derive(Debug, Clone, Copy)]
enum ExceptionReason {
    DataAbort,
    InstructionAbort,
    UnhandledSync,
    Fiq,
    Serror,
}

#[derive(Debug, Clone, Copy)]
enum SyncExceptionKind {
    Abort(ExceptionReason),
    Svc,
    Unhandled,
}

impl ExceptionReason {
    #[inline(always)]
    fn labels(self) -> (&'static str, &'static str) {
        match self {
            Self::DataAbort => ("data-abort", "data-abort"),
            Self::InstructionAbort => ("instruction-abort", "instruction-abort"),
            Self::UnhandledSync => ("unhandled-sync", "unhandled-sync"),
            Self::Fiq => ("fiq", "FIQ"),
            Self::Serror => ("serror", "SError"),
        }
    }

    #[inline(always)]
    fn as_str(self) -> &'static str {
        self.labels().0
    }

    #[inline(always)]
    fn display_name(self) -> &'static str {
        self.labels().1
    }
}

#[inline(always)]
fn read_esr_el1() -> u64 {
    let esr: u64;
    unsafe {
        core::arch::asm!("mrs {}, esr_el1", out(reg) esr);
    }
    esr
}

#[inline(always)]
fn read_far_el1() -> u64 {
    let far: u64;
    unsafe {
        core::arch::asm!("mrs {}, far_el1", out(reg) far);
    }
    far
}

#[inline(always)]
fn decode_esr_ec(esr: u64) -> u64 {
    (esr >> ESR_EC_SHIFT) & ESR_EC_MASK
}

#[inline(always)]
fn classify_sync_exception(ec: u64) -> SyncExceptionKind {
    match ec {
        ESR_EC_DABORT_LOWER_EL | ESR_EC_DABORT_CURRENT_EL => {
            SyncExceptionKind::Abort(ExceptionReason::DataAbort)
        }
        ESR_EC_IABORT_LOWER_EL | ESR_EC_IABORT_CURRENT_EL => {
            SyncExceptionKind::Abort(ExceptionReason::InstructionAbort)
        }
        ESR_EC_SVC64 => SyncExceptionKind::Svc,
        _ => SyncExceptionKind::Unhandled,
    }
}

#[inline(always)]
fn try_handle_user_page_fault(far: u64) -> bool {
    #[cfg(feature = "paging_enable")]
    {
        return crate::kernel::vmm::handle_user_page_fault(far).is_ok();
    }

    #[cfg(not(feature = "paging_enable"))]
    {
        let _ = far;
        false
    }
}

fn handle_sync_abort(frame: &ExceptionFrame, ec: u64, far: u64, reason: ExceptionReason) -> bool {
    if is_lower_el_exception(frame) {
        if try_handle_user_page_fault(far) {
            return true;
        }
        USER_ABORT_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
        handle_user_fault(reason.as_str(), ec, far, frame.elr, false);
    }

    KERNEL_ABORT_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
    handle_kernel_fault(
        reason.as_str(),
        ec,
        far,
        frame.elr,
        AARCH64_EXCEPTION_PANIC_ON_KERNEL_SYNC,
    );
}

fn handle_async_fatal(frame: &ExceptionFrame, reason: ExceptionReason, ec: u64) -> ! {
    crate::klog_error!(
        "AArch64 {}: ec={:#x} elr={:#x} spsr={:#x}",
        reason.display_name(),
        ec,
        frame.elr,
        frame.spsr
    );

    if is_lower_el_exception(frame) {
        USER_FATAL_ASYNC_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
        handle_user_fault(reason.as_str(), ec, FAR_NOT_AVAILABLE, frame.elr, true);
    }

    KERNEL_FATAL_ASYNC_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
    handle_kernel_fault(
        reason.as_str(),
        ec,
        FAR_NOT_AVAILABLE,
        frame.elr,
        AARCH64_EXCEPTION_PANIC_ON_KERNEL_ASYNC,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_sync(frame: &mut ExceptionFrame) {
    SYNC_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);

    let esr = read_esr_el1();
    let far = read_far_el1();

    use crate::kernel::bit_utils::io::aarch64_sys as esr_bits;
    let ec = esr_bits::ESR_EC.read(esr as u32) as u64;
    let iss = esr_bits::ESR_ISS.read(esr as u32) as u64;

    sync_diagnostics::record_sync_diagnostics(frame, far, esr, ec, iss);

    match classify_sync_exception(ec) {
        SyncExceptionKind::Abort(reason) => {
            if handle_sync_abort(frame, ec, far, reason) {
                return;
            }
        }
        SyncExceptionKind::Svc => {
            crate::klog_warn!("SVC call: {}", iss);
            frame.elr += AARCH64_INSTRUCTION_WIDTH_BYTES;
        }
        SyncExceptionKind::Unhandled => {
            if is_lower_el_exception(frame) {
                USER_FATAL_SYNC_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
                handle_user_fault(
                    ExceptionReason::UnhandledSync.as_str(),
                    ec,
                    far,
                    frame.elr,
                    false,
                );
            }
            handle_kernel_fault(
                ExceptionReason::UnhandledSync.as_str(),
                ec,
                far,
                frame.elr,
                AARCH64_EXCEPTION_PANIC_ON_KERNEL_SYNC,
            );
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_fiq(frame: &mut ExceptionFrame) {
    FIQ_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
    let ec = decode_esr_ec(read_esr_el1());
    handle_async_fatal(frame, ExceptionReason::Fiq, ec);
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_serror(frame: &mut ExceptionFrame) {
    SERROR_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
    let ec = decode_esr_ec(read_esr_el1());
    handle_async_fatal(frame, ExceptionReason::Serror, ec);
}
