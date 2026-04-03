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
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use limine::SmpRequest;

use crate::hal::common::ipi::wait_for_ready_count;
use crate::hal::common::smp::{wait_stats as build_wait_stats, SmpWaitStats};
#[cfg(feature = "ring_protection")]
use crate::hal::x86_64::syscalls;
use crate::hal::x86_64::apic;
use crate::interfaces::task::CpuId;
use crate::kernel::cpu_local::CpuLocal;
use crate::kernel::sync::IrqSafeMutex;

mod storage;
mod tlb;

use storage::{allocate_ap_cpu_local, allocate_ap_gdt_bundle, ap_kernel_stack_top};

#[cfg(feature = "ring_protection")]
pub(crate) use storage::allocate_kernel_stack_top;

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

pub const IPI_TLB_SHOOTDOWN_VECTOR: u8 = tlb::IPI_TLB_SHOOTDOWN_VECTOR;

/// Broadcast a TLB shootdown IPI to all other CPUs.
pub fn broadcast_tlb_shootdown(addr: u64) {
    tlb::broadcast_tlb_shootdown(
        addr,
        cpu_count(),
        TLB_SHOOTDOWN_TIMEOUT_SPINS,
        &TLB_SHOOTDOWN_TIMEOUTS,
    );
}

/// Handler for the TLB shootdown IPI.
pub extern "x86-interrupt" fn tlb_shootdown_handler(
    _stack_frame: x86_64::structures::idt::InterruptStackFrame,
) {
    tlb::handle_tlb_shootdown();
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
