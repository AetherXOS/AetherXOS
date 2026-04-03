#[cfg(target_os = "none")]
use crate::hal::common::exception::{classify_exception, ExceptionDescriptor};
#[cfg(target_os = "none")]
use crate::hal::common::irq_catalog::{classify_irq_line, IrqLineDescriptor, IrqLineKind};
#[cfg(target_os = "none")]
use crate::kernel::syscalls::syscalls_consts::x86;

#[cfg(target_os = "none")]
pub(super) const EXTENDED_IRQ_COUNT: u8 = 15;

#[cfg(target_os = "none")]
const IRQ_VECTOR_DESCRIPTORS: [IrqLineDescriptor<u8>; 2] = [
    IrqLineDescriptor::new(x86::IRQ_TIMER, IrqLineKind::Timer, "timer"),
    IrqLineDescriptor::new(x86::IRQ_TLB_SHOOTDOWN, IrqLineKind::TlbShootdown, "tlb-shootdown"),
];

#[cfg(target_os = "none")]
const EXCEPTION_DESCRIPTORS: [ExceptionDescriptor<u8>; 4] = [
    ExceptionDescriptor::new(x86::EXCEPTION_BREAKPOINT, "breakpoint"),
    ExceptionDescriptor::new(x86::EXCEPTION_DOUBLE_FAULT, "double_fault"),
    ExceptionDescriptor::new(x86::EXCEPTION_PAGE_FAULT, "page_fault"),
    ExceptionDescriptor::new(x86::EXCEPTION_GPF, "gpf"),
];

#[cfg(target_os = "none")]
#[inline(always)]
pub(super) fn exception_descriptor(vector: u8) -> ExceptionDescriptor<u8> {
    classify_exception(vector, &EXCEPTION_DESCRIPTORS, "exception")
}

#[cfg(target_os = "none")]
#[inline(always)]
pub(super) fn irq_descriptor(vector: u8) -> IrqLineDescriptor<u8> {
    classify_irq_line(vector, &IRQ_VECTOR_DESCRIPTORS, "generic")
}
