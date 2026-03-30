use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub mod selectors;
pub mod stacks;

pub use selectors::Selectors;
pub use stacks::*;

pub use crate::kernel::bit_utils::x86_64_arch::{DOUBLE_FAULT_IST_INDEX, PAGE_FAULT_IST_INDEX};
use crate::kernel::bit_utils::x86_64_arch::{DOUBLE_FAULT_STACK_SIZE, PAGE_FAULT_STACK_SIZE};

static BOOTSTRAP_GDT_READY: AtomicBool = AtomicBool::new(false);
struct StaticCell<T>(UnsafeCell<MaybeUninit<T>>);

unsafe impl<T> Sync for StaticCell<T> {}

impl<T> StaticCell<T> {
    const fn uninit() -> Self {
        Self(UnsafeCell::new(MaybeUninit::uninit()))
    }

    unsafe fn write(&self, value: T) {
        unsafe { (*self.0.get()).write(value) };
    }

    unsafe fn as_mut_ptr(&self) -> *mut T {
        unsafe { (*self.0.get()).as_mut_ptr() }
    }
}

static BOOTSTRAP_GDT_TSS: StaticCell<GdtTss> = StaticCell::uninit();
static AP_GDT_READY_MASK: AtomicU64 = AtomicU64::new(0);
static mut AP_GDT_TSS: [MaybeUninit<GdtTss>; crate::generated_consts::KERNEL_MAX_CPUS] =
    [const { MaybeUninit::uninit() }; crate::generated_consts::KERNEL_MAX_CPUS];

/// Bundle holding the GDT and TSS for a specific CPU.
pub struct GdtTss {
    pub gdt: GlobalDescriptorTable,
    pub tss: TaskStateSegment,
    pub selectors: Selectors,
}

impl GdtTss {
    pub fn new() -> Self {
        Self::new_with_ist(
            stack_end_from_static(core::ptr::addr_of!(DOUBLE_FAULT_IST_STACK) as *const u8, IST_STACK_SIZE),
            stack_end_from_static(core::ptr::addr_of!(PAGE_FAULT_IST_STACK) as *const u8, IST_STACK_SIZE),
        )
    }

    fn new_with_ist(double_fault_stack_end: VirtAddr, page_fault_stack_end: VirtAddr) -> Self {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = double_fault_stack_end;
        tss.interrupt_stack_table[PAGE_FAULT_IST_INDEX as usize] = page_fault_stack_end;

        let mut gdt = GlobalDescriptorTable::new();
        let kernel_code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let kernel_data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
        let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());

        Self {
            gdt,
            tss,
            selectors: Selectors {
                kernel_code_selector,
                kernel_data_selector,
                user_code_selector,
                user_data_selector,
                tss_selector: SegmentSelector::new(0, x86_64::PrivilegeLevel::Ring0),
            },
        }
    }

    pub unsafe fn load(&'static mut self) {
        use x86_64::instructions::segmentation::{Segment, CS, DS, ES, SS};
        use x86_64::instructions::tables::load_tss;

        let tss_ref: &'static TaskStateSegment = unsafe { core::mem::transmute(&self.tss) };
        self.selectors.tss_selector = self.gdt.add_entry(Descriptor::tss_segment(tss_ref));

        self.gdt.load();

        unsafe { CS::set_reg(self.selectors.kernel_code_selector) };
        unsafe { load_tss(self.selectors.tss_selector) };

        unsafe { SS::set_reg(self.selectors.kernel_data_selector) };
        unsafe { DS::set_reg(self.selectors.kernel_data_selector) };
        unsafe { ES::set_reg(self.selectors.kernel_data_selector) };
    }
}

pub unsafe fn bootstrap_gdt_tss() -> &'static mut GdtTss {
    if !BOOTSTRAP_GDT_READY.load(Ordering::Acquire) {
        unsafe { BOOTSTRAP_GDT_TSS.write(GdtTss::new()); }
        BOOTSTRAP_GDT_READY.store(true, Ordering::Release);
    }
    unsafe { &mut *BOOTSTRAP_GDT_TSS.as_mut_ptr() }
}

pub unsafe fn ap_gdt_tss(cpu_id: crate::interfaces::task::CpuId) -> &'static mut GdtTss {
    let slot = cpu_id.0;
    assert!(slot < crate::generated_consts::KERNEL_MAX_CPUS);
    let bit = 1u64 << slot;
    if AP_GDT_READY_MASK.load(Ordering::Acquire) & bit == 0 {
        unsafe {
            AP_GDT_TSS[slot].write(GdtTss::new_with_ist(
                stack_end_from_static(core::ptr::addr_of!(AP_DOUBLE_FAULT_IST_STACKS[slot]) as *const u8, IST_STACK_SIZE),
                stack_end_from_static(core::ptr::addr_of!(AP_PAGE_FAULT_IST_STACKS[slot]) as *const u8, IST_STACK_SIZE),
            ));
        }
        AP_GDT_READY_MASK.fetch_or(bit, Ordering::Release);
    }
    unsafe { &mut *AP_GDT_TSS[slot].as_mut_ptr() }
}
