use core::cell::UnsafeCell;
use x86_64::VirtAddr;
use crate::kernel::bit_utils::x86_64_arch::DOUBLE_FAULT_STACK_SIZE;

/// Size of the IST interrupt stacks (legacy constant, now redirected to bit_utils).
pub const IST_STACK_SIZE: usize = DOUBLE_FAULT_STACK_SIZE;
const MAX_CPUS: usize = crate::generated_consts::KERNEL_MAX_CPUS;

#[inline(always)]
pub fn stack_end_from_static(stack: *const u8, size: usize) -> VirtAddr {
    VirtAddr::from_ptr(stack) + size
}

struct StaticIstStack(UnsafeCell<[u8; IST_STACK_SIZE]>);
struct StaticIstStackArray(UnsafeCell<[[u8; IST_STACK_SIZE]; MAX_CPUS]>);

unsafe impl Sync for StaticIstStack {}
unsafe impl Sync for StaticIstStackArray {}

impl StaticIstStack {
    const fn zeroed() -> Self {
        Self(UnsafeCell::new([0u8; IST_STACK_SIZE]))
    }

    fn ptr(&self) -> *const u8 {
        self.0.get() as *const u8
    }
}

impl StaticIstStackArray {
    const fn zeroed() -> Self {
        Self(UnsafeCell::new([[0u8; IST_STACK_SIZE]; MAX_CPUS]))
    }

    fn slot_ptr(&self, slot: usize) -> *const u8 {
        let base = self.0.get() as *const [u8; IST_STACK_SIZE];
        unsafe { base.add(slot) as *const u8 }
    }
}

static DOUBLE_FAULT_IST_STACK: StaticIstStack = StaticIstStack::zeroed();
static PAGE_FAULT_IST_STACK: StaticIstStack = StaticIstStack::zeroed();

static AP_DOUBLE_FAULT_IST_STACKS: StaticIstStackArray = StaticIstStackArray::zeroed();
static AP_PAGE_FAULT_IST_STACKS: StaticIstStackArray = StaticIstStackArray::zeroed();

#[inline(always)]
pub fn bootstrap_double_fault_stack_ptr() -> *const u8 {
    DOUBLE_FAULT_IST_STACK.ptr()
}

#[inline(always)]
pub fn bootstrap_page_fault_stack_ptr() -> *const u8 {
    PAGE_FAULT_IST_STACK.ptr()
}

#[inline(always)]
pub fn ap_double_fault_stack_ptr(slot: usize) -> *const u8 {
    AP_DOUBLE_FAULT_IST_STACKS.slot_ptr(slot)
}

#[inline(always)]
pub fn ap_page_fault_stack_ptr(slot: usize) -> *const u8 {
    AP_PAGE_FAULT_IST_STACKS.slot_ptr(slot)
}
