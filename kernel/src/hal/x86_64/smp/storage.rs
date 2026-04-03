use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::hal::x86_64::gdt;
use crate::interfaces::task::CpuId;
use crate::kernel::cpu_local::CpuLocal;

const KERNEL_STACK_BYTES: usize = crate::generated_consts::STACK_SIZE_PAGES * 4096;
#[cfg(feature = "ring_protection")]
const BOOTSTRAP_LAUNCH_STACK_SLOTS: usize = 8;
const MAX_CPUS: usize = crate::generated_consts::KERNEL_MAX_CPUS;

struct StaticCpuLocals(UnsafeCell<[MaybeUninit<CpuLocal>; MAX_CPUS]>);

unsafe impl Sync for StaticCpuLocals {}

impl StaticCpuLocals {
    const fn uninit() -> Self {
        Self(UnsafeCell::new([const { MaybeUninit::uninit() }; MAX_CPUS]))
    }

    unsafe fn write_slot(&self, slot: usize, value: CpuLocal) {
        unsafe {
            (*self.0.get())[slot].write(value);
        }
    }

    unsafe fn slot_ptr(&self, slot: usize) -> *const CpuLocal {
        unsafe { (*self.0.get())[slot].as_ptr() }
    }
}

struct StaticKernelStacks<const SLOTS: usize>(UnsafeCell<[[u8; KERNEL_STACK_BYTES]; SLOTS]>);

unsafe impl<const SLOTS: usize> Sync for StaticKernelStacks<SLOTS> {}

impl<const SLOTS: usize> StaticKernelStacks<SLOTS> {
    const fn zeroed() -> Self {
        Self(UnsafeCell::new([[0u8; KERNEL_STACK_BYTES]; SLOTS]))
    }

    fn slot_base_addr(&self, slot: usize) -> usize {
        let base = self.0.get() as *const [u8; KERNEL_STACK_BYTES];
        unsafe { base.add(slot) as *const u8 as usize }
    }
}

static AP_CPU_LOCAL_READY_MASK: AtomicU64 = AtomicU64::new(0);
static AP_CPU_LOCAL: StaticCpuLocals = StaticCpuLocals::uninit();
static AP_KERNEL_STACKS: StaticKernelStacks<{ crate::generated_consts::KERNEL_MAX_CPUS }> =
    StaticKernelStacks::zeroed();
#[cfg(feature = "ring_protection")]
static BOOTSTRAP_LAUNCH_STACKS: StaticKernelStacks<BOOTSTRAP_LAUNCH_STACK_SLOTS> =
    StaticKernelStacks::zeroed();
#[cfg(feature = "ring_protection")]
static NEXT_BOOTSTRAP_LAUNCH_STACK_SLOT: AtomicUsize = AtomicUsize::new(0);

pub(super) fn ap_kernel_stack_top(slot: usize) -> usize {
    let top = AP_KERNEL_STACKS.slot_base_addr(slot) + KERNEL_STACK_BYTES;
    top & !0xF
}

#[cfg(feature = "ring_protection")]
pub(crate) fn allocate_kernel_stack_top() -> usize {
    let slot = NEXT_BOOTSTRAP_LAUNCH_STACK_SLOT.fetch_add(1, Ordering::Relaxed);
    if slot < BOOTSTRAP_LAUNCH_STACK_SLOTS {
        let top = BOOTSTRAP_LAUNCH_STACKS.slot_base_addr(slot) + KERNEL_STACK_BYTES;
        return top & !0xF;
    }
    let stack = alloc::vec![0u8; KERNEL_STACK_BYTES].into_boxed_slice();
    let top = stack.as_ptr() as usize + KERNEL_STACK_BYTES;
    let aligned = top & !0xF;
    let _ = alloc::boxed::Box::leak(stack);
    aligned
}

#[inline(never)]
pub(super) fn allocate_ap_gdt_bundle(cpu_id: CpuId) -> &'static mut gdt::GdtTss {
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap gdt heap alloc begin\n");
    let bundle = unsafe { gdt::ap_gdt_tss(cpu_id) };
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap gdt heap alloc returned\n");
    bundle
}

#[inline(never)]
pub(super) fn allocate_ap_cpu_local(cpu_id: CpuId) -> &'static CpuLocal {
    let slot = cpu_id.0;
    assert!(slot < MAX_CPUS);
    let bit = 1u64 << slot;
    if AP_CPU_LOCAL_READY_MASK.load(Ordering::Acquire) & bit != 0 {
        return unsafe { &*AP_CPU_LOCAL.slot_ptr(slot) };
    }
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap scheduler create begin\n");
    let scheduler = crate::modules::selector::bootstrap_active_scheduler();
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap scheduler create returned\n");
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap scheduler mutex begin\n");
    let scheduler = crate::kernel::sync::IrqSafeMutex::new(scheduler);
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap scheduler mutex returned\n");
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap cpu local heap alloc begin\n");
    unsafe {
        AP_CPU_LOCAL.write_slot(slot, CpuLocal {
            cpu_id,
            #[cfg(feature = "ring_protection")]
            scratch: 0,
            #[cfg(feature = "ring_protection")]
            kernel_stack_top: core::sync::atomic::AtomicUsize::new(ap_kernel_stack_top(slot)),
            current_task: core::sync::atomic::AtomicUsize::new(0),
            heartbeat_tick: core::sync::atomic::AtomicU64::new(0),
            idle_stack_pointer: core::sync::atomic::AtomicUsize::new(0),
            scheduler,
        });
    }
    AP_CPU_LOCAL_READY_MASK.fetch_or(bit, Ordering::Release);
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap cpu local heap alloc returned\n");
    unsafe { &*AP_CPU_LOCAL.slot_ptr(slot) }
}
