use x86_64::VirtAddr;
use crate::kernel::bit_utils::x86_64_arch::{DOUBLE_FAULT_STACK_SIZE, PAGE_FAULT_STACK_SIZE};

/// Size of the IST interrupt stacks (legacy constant, now redirected to bit_utils).
pub const IST_STACK_SIZE: usize = DOUBLE_FAULT_STACK_SIZE;

#[inline(always)]
pub fn stack_end_from_static(stack: *const u8, size: usize) -> VirtAddr {
    VirtAddr::from_ptr(stack) + size
}

pub static mut DOUBLE_FAULT_IST_STACK: [u8; IST_STACK_SIZE] = [0u8; IST_STACK_SIZE];
pub static mut PAGE_FAULT_IST_STACK: [u8; IST_STACK_SIZE] = [0u8; IST_STACK_SIZE];

pub static mut AP_DOUBLE_FAULT_IST_STACKS: [[u8; IST_STACK_SIZE]; crate::generated_consts::KERNEL_MAX_CPUS] =
    [[0u8; IST_STACK_SIZE]; crate::generated_consts::KERNEL_MAX_CPUS];
pub static mut AP_PAGE_FAULT_IST_STACKS: [[u8; IST_STACK_SIZE]; crate::generated_consts::KERNEL_MAX_CPUS] =
    [[0u8; IST_STACK_SIZE]; crate::generated_consts::KERNEL_MAX_CPUS];
