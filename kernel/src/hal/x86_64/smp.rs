/// Symmetric Multi-Processing — x86_64 / Limine Boot Protocol.
///
/// Limine enumerates all processors and provides an SmpInfo per core.
/// For each AP we:
///   1. Load a fresh GDT+TSS
///   2. Init syscall MSRs (ring_protection only)
///   3. Init Local APIC (enable timer + EOI)
///   4. Bootstrap per-CPU CpuLocal struct
///   5. Register with the global CPU list
///   6. Enable interrupts and enter idle loop
use alloc::vec::Vec;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use limine::SmpRequest;

use crate::hal::common::ipi::{acknowledge_pending, wait_for_pending_acks, wait_for_ready_count};
use crate::hal::common::smp::{wait_stats as build_wait_stats, SmpWaitStats};
#[cfg(feature = "ring_protection")]
use crate::hal::x86_64::syscalls;
use crate::hal::x86_64::{apic, gdt};
use crate::interfaces::task::CpuId;
use crate::kernel::cpu_local::CpuLocal;
use crate::kernel::sync::IrqSafeMutex;

// ── Limine SMP request ────────────────────────────────────────────────────────

#[used]
static SMP_REQUEST: SmpRequest = SmpRequest::new(0);

// ── AP readiness tracking ─────────────────────────────────────────────────────

/// Number of APs that have successfully initialised.
static AP_ONLINE_COUNT: AtomicU32 = AtomicU32::new(0);
/// Total APs Limine found (set during init()).
static AP_TOTAL_COUNT: AtomicU32 = AtomicU32::new(0);
static AP_BOOT_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
static TLB_SHOOTDOWN_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
const AP_BOOT_TIMEOUT_SPINS: usize = 50_000_000;
const TLB_SHOOTDOWN_TIMEOUT_SPINS: usize = 2_000_000;

// ── Kernel stack allocation ───────────────────────────────────────────────────

const KERNEL_STACK_BYTES: usize = crate::generated_consts::STACK_SIZE_PAGES * 4096;
#[cfg(feature = "ring_protection")]
const BOOTSTRAP_LAUNCH_STACK_SLOTS: usize = 8;
static AP_CPU_LOCAL_READY_MASK: AtomicU64 = AtomicU64::new(0);
static mut AP_CPU_LOCAL: [MaybeUninit<CpuLocal>; crate::generated_consts::KERNEL_MAX_CPUS] =
    [const { MaybeUninit::uninit() }; crate::generated_consts::KERNEL_MAX_CPUS];
static mut AP_KERNEL_STACKS: [[u8; KERNEL_STACK_BYTES]; crate::generated_consts::KERNEL_MAX_CPUS] =
    [[0u8; KERNEL_STACK_BYTES]; crate::generated_consts::KERNEL_MAX_CPUS];
#[cfg(feature = "ring_protection")]
static mut BOOTSTRAP_LAUNCH_STACKS: [[u8; KERNEL_STACK_BYTES]; BOOTSTRAP_LAUNCH_STACK_SLOTS] =
    [[0u8; KERNEL_STACK_BYTES]; BOOTSTRAP_LAUNCH_STACK_SLOTS];
#[cfg(feature = "ring_protection")]
static NEXT_BOOTSTRAP_LAUNCH_STACK_SLOT: AtomicUsize = AtomicUsize::new(0);

fn ap_kernel_stack_top(slot: usize) -> usize {
    let top = unsafe {
        (core::ptr::addr_of!(AP_KERNEL_STACKS[slot]) as *const u8 as usize) + KERNEL_STACK_BYTES
    };
    top & !0xF
}

/// Allocate a 16-byte-aligned kernel interrupt stack and return its top.
#[cfg(feature = "ring_protection")]
pub(crate) fn allocate_kernel_stack_top() -> usize {
    let slot = NEXT_BOOTSTRAP_LAUNCH_STACK_SLOT.fetch_add(1, Ordering::Relaxed);
    if slot < BOOTSTRAP_LAUNCH_STACK_SLOTS {
        let top = unsafe {
            (core::ptr::addr_of!(BOOTSTRAP_LAUNCH_STACKS[slot]) as *const u8 as usize)
                + KERNEL_STACK_BYTES
        };
        return top & !0xF;
    }
    let stack = alloc::vec![0u8; KERNEL_STACK_BYTES].into_boxed_slice();
    let top = stack.as_ptr() as usize + KERNEL_STACK_BYTES;
    let aligned = top & !0xF;
    let _ = alloc::boxed::Box::leak(stack);
    aligned
}

#[inline(never)]
fn allocate_ap_gdt_bundle(cpu_id: CpuId) -> &'static mut gdt::GdtTss {
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap gdt heap alloc begin\n");
    let bundle = unsafe { gdt::ap_gdt_tss(cpu_id) };
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap gdt heap alloc returned\n");
    bundle
}

#[inline(never)]
fn allocate_ap_cpu_local(cpu_id: CpuId) -> &'static CpuLocal {
    let slot = cpu_id.0;
    assert!(slot < crate::generated_consts::KERNEL_MAX_CPUS);
    let bit = 1u64 << slot;
    if AP_CPU_LOCAL_READY_MASK.load(Ordering::Acquire) & bit != 0 {
        return unsafe { &*AP_CPU_LOCAL[slot].as_ptr() };
    }
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap scheduler create begin\n");
    let scheduler = crate::modules::selector::bootstrap_active_scheduler();
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap scheduler create returned\n");
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap scheduler mutex begin\n");
    let scheduler = crate::kernel::sync::IrqSafeMutex::new(scheduler);
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap scheduler mutex returned\n");
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap cpu local heap alloc begin\n");
    unsafe {
        AP_CPU_LOCAL[slot].write(CpuLocal {
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
    unsafe { &*AP_CPU_LOCAL[slot].as_ptr() }
}

// ── Global CPU registry ───────────────────────────────────────────────────────

/// List of all online CPU locals (BSP + APs).
pub static CPUS: IrqSafeMutex<Vec<&'static CpuLocal>> = IrqSafeMutex::new(Vec::new());

pub fn register_cpu(cpu: &'static CpuLocal) {
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 register_cpu push begin\n");
    CPUS.lock().push(cpu);
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 register_cpu push returned\n");
}

pub fn get_cpu_local(index: usize) -> Option<&'static CpuLocal> {
    CPUS.lock().get(index).copied()
}

static SHOOTDOWN_ADDR: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
static SHOOTDOWN_PENDING: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

pub const IPI_TLB_SHOOTDOWN_VECTOR: u8 = 253;

/// Broadcast a TLB shootdown IPI to all other CPUs.
pub fn broadcast_tlb_shootdown(addr: u64) {
    use x86_64::VirtAddr;
    let n_cpus = cpu_count();

    // Always flush locally
    x86_64::instructions::tlb::flush(VirtAddr::new(addr));

    if n_cpus <= 1 {
        return;
    }

    SHOOTDOWN_ADDR.store(addr, Ordering::Release);
    SHOOTDOWN_PENDING.store(n_cpus - 1, Ordering::Release);

    // SAFETY: Send IPI to all other cores.
    unsafe {
        apic::send_ipi_all_excluding_self(IPI_TLB_SHOOTDOWN_VECTOR);
    }

    // Wait for all CPUs to finish flushing with bounded timeout.
    if let Some(left) = wait_for_pending_acks(&SHOOTDOWN_PENDING, TLB_SHOOTDOWN_TIMEOUT_SPINS) {
        TLB_SHOOTDOWN_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
        crate::klog_warn!(
            "x86_64 TLB shootdown timeout: {} AP(s) did not respond",
            left
        );
        SHOOTDOWN_PENDING.store(0, Ordering::Release);
    }
}

/// Handler for the TLB shootdown IPI.
pub extern "x86-interrupt" fn tlb_shootdown_handler(
    _stack_frame: x86_64::structures::idt::InterruptStackFrame,
) {
    use x86_64::VirtAddr;
    let addr = SHOOTDOWN_ADDR.load(Ordering::Acquire);
    unsafe {
        x86_64::instructions::tlb::flush(VirtAddr::new(addr));
        apic::eoi();
    }
    acknowledge_pending(&SHOOTDOWN_PENDING);
}

pub fn cpu_count() -> usize {
    CPUS.lock().len()
}

pub fn ap_online_count() -> u32 {
    AP_ONLINE_COUNT.load(Ordering::Acquire)
}
pub fn ap_total_count() -> u32 {
    AP_TOTAL_COUNT.load(Ordering::Relaxed)
}

pub fn wait_stats() -> SmpWaitStats {
    build_wait_stats(
        AP_BOOT_TIMEOUT_SPINS,
        AP_BOOT_TIMEOUTS.load(Ordering::Relaxed),
        TLB_SHOOTDOWN_TIMEOUT_SPINS,
        TLB_SHOOTDOWN_TIMEOUTS.load(Ordering::Relaxed),
    )
}

// ── AP entry point ────────────────────────────────────────────────────────────

/// Limine calls this function for each AP (one call per core).
extern "C" fn ap_entry(info: *const limine::SmpInfo) -> ! {
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap entry begin\n");
    let info = unsafe { &*info };
    let cpu_id = CpuId(info.lapic_id as usize);
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap cpu id ready\n");

    // 1. Load a per-AP GDT+TSS.
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap gdt bundle begin\n");
    let gdt_bundle = allocate_ap_gdt_bundle(cpu_id);
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap gdt bundle returned\n");
    let selectors = gdt_bundle.selectors;
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap gdt load begin\n");
    unsafe {
        gdt_bundle.load();
    }
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap gdt load returned\n");

    // 1b. Load the shared IDT on this AP core.
    // The IDTR register is per-CPU; without this, any interrupt on the AP
    // will read IDT base = 0 and immediately triple-fault.
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap idt load begin\n");
    crate::hal::x86_64::idt::init();
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap idt load returned\n");

    // 2. Enable this AP's Local APIC.
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap local apic begin\n");
    unsafe {
        apic::init_local_apic();
    }
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap local apic returned\n");

    // 3. Bootstrap CpuLocal (uses the global heap — safe after BSP init_heap).
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap cpu local alloc begin\n");
    let cpu_local = allocate_ap_cpu_local(cpu_id);
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap cpu local alloc returned\n");

    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap cpu local init begin\n");
    unsafe {
        cpu_local.init();
    }
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap cpu local init returned\n");

    // 4. Set up syscall MSRs after CpuLocal/GS/kernel stack are ready.
    #[cfg(feature = "ring_protection")]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap syscalls init begin\n");
    #[cfg(feature = "ring_protection")]
    syscalls::init(&selectors);
    #[cfg(feature = "ring_protection")]
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap syscalls init returned\n");
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap register cpu begin\n");
    register_cpu(cpu_local);
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap register cpu returned\n");

    crate::klog_info!("x86_64 AP {} online (lapic_id={})", cpu_id.0, info.lapic_id);

    // Signal BSP that we are up.
    AP_ONLINE_COUNT.fetch_add(1, Ordering::Release);

    // 5. Switch from Limine's tiny bootstrap stack to our 64 KiB kernel
    //    stack, then enable interrupts and idle.
    //    Without this the AP overflows its ~4 KiB bootstrap stack within
    //    a few dozen timer ticks and triple-faults.
    let new_stack_top = ap_kernel_stack_top(cpu_id.0);
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap stack switch begin\n");
    unsafe {
        core::arch::asm!(
            "mov rsp, {0}",
            "call {1}",
            in(reg) new_stack_top,
            sym ap_idle_loop,
            options(noreturn),
        );
    }
}

/// AP idle loop — runs on the AP's own kernel stack.
/// This function is called via `call` after switching RSP, so it must
/// never return.
#[inline(never)]
fn ap_idle_loop() -> ! {
    crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] x86_64 ap idle loop entered\n");
    x86_64::instructions::interrupts::enable();
    loop {
        crate::kernel::idle_once();
    }
}

// ── BSP SMP init ─────────────────────────────────────────────────────────────

pub fn init() {
    let response_ptr = match SMP_REQUEST.get_response().as_ptr() {
        Some(p) => p,
        None => {
            crate::klog_warn!("x86_64 SMP: no SMP response from bootloader — uniprocessor mode");
            return;
        }
    };
    let response = unsafe { &mut *response_ptr };

    let cpus = response.cpus();
    let ap_count = cpus.len().saturating_sub(1) as u32; // BSP already running
    AP_TOTAL_COUNT.store(ap_count, Ordering::Relaxed);

    for smp_info in cpus {
        if smp_info.lapic_id == 0 {
            // BSP already online.
            continue;
        }
        // Write the AP entry pointer — Limine will jump to it when it sees it non-null.
        smp_info.goto_address = ap_entry;
    }

    // Wait for all APs to come online (with a generous spin limit).
    if !wait_for_ready_count(&AP_ONLINE_COUNT, ap_count, AP_BOOT_TIMEOUT_SPINS) {
        AP_BOOT_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
        let got = AP_ONLINE_COUNT.load(Ordering::Relaxed);
        crate::klog_warn!(
            "x86_64 SMP: timeout waiting for APs ({}/{} online)",
            got,
            ap_count
        );
    }

    crate::klog_info!(
        "x86_64 SMP: {}/{} APs online",
        AP_ONLINE_COUNT.load(Ordering::Relaxed),
        ap_count
    );
}
