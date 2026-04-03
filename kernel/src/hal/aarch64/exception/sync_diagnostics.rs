use super::*;
use crate::hal::common::exception::{
    record_exception_snapshot, ExceptionSnapshot,
};

#[inline(always)]
fn describe_aarch64_exception_class(ec: u64) -> &'static str {
    match ec {
        0b000000 => "Unknown Reason",
        0b000001 => "WFI/WFE Trap",
        0b000111 => "SVE/SIMD/FP Trap",
        0b001110 => "Illegal Execution State",
        0b010101 => "SVC in 64-bit state",
        0b100000 => "Instruction Abort (Lower EL)",
        0b100001 => "Instruction Abort (Current EL)",
        0b100010 => "PC Alignment Fault",
        0b100100 => "Data Abort (Lower EL)",
        0b100101 => "Data Abort (Current EL)",
        0b100110 => "SP Alignment Fault",
        0b110000 => "Breakpoint (Lower EL)",
        0b110001 => "Breakpoint (Current EL)",
        0b111100 => "BRK Instruction",
        _ => "Other/Unknown",
    }
}

pub(super) fn record_sync_diagnostics(frame: &ExceptionFrame, far: u64, esr: u64, ec: u64, iss: u64) {
    crate::klog_warn!(
        "Synchronous Exception: {} (EC: {:#08b}, ISS: {:#x})",
        describe_aarch64_exception_class(ec),
        ec,
        iss
    );
    crate::klog_warn!(
        "FAR_EL1: {:#x}, ELR_EL1: {:#x}, SPSR_EL1: {:#x}",
        far,
        frame.elr,
        frame.spsr
    );
    let bytes = unsafe {
        core::slice::from_raw_parts(
            (frame as *const ExceptionFrame).cast::<u8>(),
            core::mem::size_of::<ExceptionFrame>(),
        )
    };
    record_exception_snapshot(ExceptionSnapshot {
        trace_label: "aarch64.sync",
        dump_label: "aarch64.sync.frame",
        frame_bytes: bytes,
        instruction_pointer: frame.elr,
        stack_pointer: frame.sp_el0,
        fault_or_code: far,
        status_or_flags: esr,
    });
}
