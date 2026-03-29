use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

// Index in Interrupt Stack Table (IST)
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
pub const PAGE_FAULT_IST_INDEX: u16 = 1;
pub const SYSCALL_IST_INDEX: u16 = 2; // Separate stack for syscalls if needed (syscall uses RSP0 usually)
const IST_STACK_SIZE: usize = 4096 * 5;

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
static mut DOUBLE_FAULT_IST_STACK: [u8; IST_STACK_SIZE] = [0u8; IST_STACK_SIZE];
static mut PAGE_FAULT_IST_STACK: [u8; IST_STACK_SIZE] = [0u8; IST_STACK_SIZE];
static AP_GDT_READY_MASK: AtomicU64 = AtomicU64::new(0);
static mut AP_GDT_TSS: [MaybeUninit<GdtTss>; crate::generated_consts::KERNEL_MAX_CPUS] =
    [const { MaybeUninit::uninit() }; crate::generated_consts::KERNEL_MAX_CPUS];
static mut AP_DOUBLE_FAULT_IST_STACKS: [[u8; IST_STACK_SIZE]; crate::generated_consts::KERNEL_MAX_CPUS] =
    [[0u8; IST_STACK_SIZE]; crate::generated_consts::KERNEL_MAX_CPUS];
static mut AP_PAGE_FAULT_IST_STACKS: [[u8; IST_STACK_SIZE]; crate::generated_consts::KERNEL_MAX_CPUS] =
    [[0u8; IST_STACK_SIZE]; crate::generated_consts::KERNEL_MAX_CPUS];

#[inline(always)]
fn stack_end_from_static(stack: *const u8, size: usize) -> VirtAddr {
    VirtAddr::from_ptr(stack) + size
}

/// Selectors used for Kernel/User transitions
#[derive(Debug, Clone, Copy)]
pub struct Selectors {
    pub kernel_code_selector: SegmentSelector,
    pub kernel_data_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
}

/// Bundle holding the GDT and TSS for a specific CPU.
/// This must be kept alive as long as the CPU is running (forever).
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
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] gdt new begin\n");
        let mut tss = TaskStateSegment::new();
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] gdt tss created\n");

        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = double_fault_stack_end;
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] gdt double fault ist ready\n");

        tss.interrupt_stack_table[PAGE_FAULT_IST_INDEX as usize] = page_fault_stack_end;
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] gdt page fault ist ready\n");

        let mut gdt = GlobalDescriptorTable::new();
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] gdt table created\n");

        let kernel_code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let kernel_data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
        let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] gdt selectors added\n");

        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] gdt new returning\n");
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

    /// Finalize initialization and load the tables.
    /// MUST be called after this struct is moved to its final location (e.g., Boxed or in static array).
    pub unsafe fn load(&'static mut self) {
        use x86_64::instructions::segmentation::{Segment, CS, DS, ES, SS};
        use x86_64::instructions::tables::load_tss;

        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] gdt load begin\n");
        // 1. Add TSS to GDT now that its address is stable (using self.tss)
        // Safety: `self` is pinned for the remainder of execution, so the TSS address is stable.
        let tss_ref: &'static TaskStateSegment = unsafe { core::mem::transmute(&self.tss) };
        self.selectors.tss_selector = self.gdt.add_entry(Descriptor::tss_segment(tss_ref));
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] gdt tss descriptor added\n");

        // 2. Load GDT
        self.gdt.load();
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] gdt table loaded\n");

        // 3. Load CS and TSS
        // Safety: selectors were created from the GDT loaded immediately above.
        unsafe { CS::set_reg(self.selectors.kernel_code_selector) };
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] gdt cs loaded\n");
        // Safety: TSS selector points at the stable TSS descriptor inserted above.
        unsafe { load_tss(self.selectors.tss_selector) };
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] gdt tss loaded\n");

        // 4. Load Data Segments
        // Safety: selectors were created from the GDT loaded immediately above.
        unsafe { SS::set_reg(self.selectors.kernel_data_selector) };
        unsafe { DS::set_reg(self.selectors.kernel_data_selector) };
        unsafe { ES::set_reg(self.selectors.kernel_data_selector) };
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] gdt data segments loaded\n");

        // Clear FS/GS (GS will be set by CpuLocal logic later, FS unused)
        // FS::set_reg(SegmentSelector(0)); // Null
    }
}

pub unsafe fn bootstrap_gdt_tss() -> &'static mut GdtTss {
    if !BOOTSTRAP_GDT_READY.load(Ordering::Acquire) {
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] bootstrap gdt init begin\n");
        unsafe {
            BOOTSTRAP_GDT_TSS.write(GdtTss::new());
        }
        BOOTSTRAP_GDT_READY.store(true, Ordering::Release);
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] bootstrap gdt init returned\n");
    }

    unsafe { &mut *BOOTSTRAP_GDT_TSS.as_mut_ptr() }
}

pub unsafe fn ap_gdt_tss(cpu_id: crate::interfaces::task::CpuId) -> &'static mut GdtTss {
    let slot = cpu_id.0;
    assert!(slot < crate::generated_consts::KERNEL_MAX_CPUS);
    let bit = 1u64 << slot;
    if AP_GDT_READY_MASK.load(Ordering::Acquire) & bit == 0 {
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] ap gdt slot write begin\n");
        unsafe {
            AP_GDT_TSS[slot].write(GdtTss::new_with_ist(
                stack_end_from_static(core::ptr::addr_of!(AP_DOUBLE_FAULT_IST_STACKS[slot]) as *const u8, IST_STACK_SIZE),
                stack_end_from_static(core::ptr::addr_of!(AP_PAGE_FAULT_IST_STACKS[slot]) as *const u8, IST_STACK_SIZE),
            ));
        }
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] ap gdt slot write returned\n");
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] ap gdt ready mask set begin\n");
        AP_GDT_READY_MASK.fetch_or(bit, Ordering::Release);
        #[cfg(target_arch = "x86_64")]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] ap gdt ready mask set returned\n");
    }

    #[cfg(target_arch = "x86_64")]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] ap gdt slot return begin\n");
    unsafe { &mut *AP_GDT_TSS[slot].as_mut_ptr() }
}
