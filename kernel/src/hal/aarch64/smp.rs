/// AArch64 Symmetric Multi-Processing initialisation.
///
/// On AArch64 with PSCI, application processors (APs) are brought online via
/// the `CPU_ON` PSCI call.  We iterate over the MPIDR values collected during
/// ACPI/DTB discovery and wake each core, directing it to `aarch64_ap_entry`.
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

use crate::generated_consts::{
    AARCH64_SMP_BOOT_TIMEOUT_SPINS, AARCH64_SMP_KNOWN_MPIDRS, AARCH64_TLB_SHOOTDOWN_TIMEOUT_SPINS,
};
use crate::hal::common::ipi::{acknowledge_pending, wait_for_pending_acks, wait_for_ready_count};
use crate::hal::common::smp::{wait_stats as build_wait_stats, SmpWaitStats};
use crate::interfaces::task::CpuId;
use crate::kernel::cpu_local::CpuLocal;
use crate::kernel::sync::IrqSafeMutex;

// ── GIC SGI vector for TLB shootdown ─────────────────────────────────────────
/// SGI 15 is reserved for TLB invalidation IPIs.
pub const SGI_TLB_SHOOTDOWN: u8 = 15;
const ICC_SGI1R_IRM_BIT: u64 = 1u64 << 40;
const ICC_SGI1R_INTID_SHIFT: u64 = 24;
const GICD_SGIR_TARGET_ALL_BUT_SELF: u32 = 0b01u32 << 24;
const MPIDR_AFF0_MASK: u64 = 0xFF;
const DAIFCLR_UNMASK_ALL: u8 = 0xF;

// ── Global CPU state ──────────────────────────────────────────────────────────

lazy_static::lazy_static! {
    /// Number of cores that have successfully initialised.
    pub static ref ACCESSIBLE_CORES: IrqSafeMutex<usize> = IrqSafeMutex::new(1);
    /// Pointers to per-CPU structs for all online cores.
    pub static ref CPUS: IrqSafeMutex<Vec<&'static CpuLocal>> = IrqSafeMutex::new(Vec::new());
}

// ── Kernel stack allocation ───────────────────────────────────────────────────

#[cfg(feature = "ring_protection")]
const KERNEL_STACK_BYTES: usize = crate::generated_consts::STACK_SIZE_PAGES * 4096;
#[cfg(feature = "ring_protection")]
const BOOTSTRAP_LAUNCH_STACK_SLOTS: usize = 8;
#[cfg(feature = "ring_protection")]
struct StaticLaunchStacks(UnsafeCell<[[u8; KERNEL_STACK_BYTES]; BOOTSTRAP_LAUNCH_STACK_SLOTS]>);

#[cfg(feature = "ring_protection")]
unsafe impl Sync for StaticLaunchStacks {}

#[cfg(feature = "ring_protection")]
impl StaticLaunchStacks {
    const fn zeroed() -> Self {
        Self(UnsafeCell::new(
            [[0u8; KERNEL_STACK_BYTES]; BOOTSTRAP_LAUNCH_STACK_SLOTS],
        ))
    }

    fn slot_base_addr(&self, slot: usize) -> usize {
        let base = self.0.get() as *const [u8; KERNEL_STACK_BYTES];
        unsafe { base.add(slot) as *const u8 as usize }
    }
}

#[cfg(feature = "ring_protection")]
static BOOTSTRAP_LAUNCH_STACKS: StaticLaunchStacks = StaticLaunchStacks::zeroed();
#[cfg(feature = "ring_protection")]
static NEXT_BOOTSTRAP_LAUNCH_STACK_SLOT: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "ring_protection")]
pub fn allocate_kernel_stack_top() -> usize {
    let slot = NEXT_BOOTSTRAP_LAUNCH_STACK_SLOT.fetch_add(1, Ordering::Relaxed);
    if slot < BOOTSTRAP_LAUNCH_STACK_SLOTS {
        let top = BOOTSTRAP_LAUNCH_STACKS.slot_base_addr(slot) + KERNEL_STACK_BYTES;
        return top & !0xF;
    }
    let stack = alloc::vec![0u8; KERNEL_STACK_BYTES].into_boxed_slice();
    let top = stack.as_ptr() as usize + KERNEL_STACK_BYTES;
    let aligned_top = top & !0xF;
    let _ = Box::leak(stack);
    aligned_top
}

#[cfg(not(feature = "ring_protection"))]
pub fn allocate_kernel_stack_top() -> usize {
    0
}

// ── AP registry ──────────────────────────────────────────────────────────────

pub fn register_cpu(local: &'static CpuLocal) {
    CPUS.lock().push(local);
    *ACCESSIBLE_CORES.lock() += 1;
}

pub fn cpu_count() -> usize {
    *ACCESSIBLE_CORES.lock()
}

// ── AArch64 TLB Shootdown ─────────────────────────────────────────────────────
//
// Supports two GIC generations:
//   GICv2: Write to GICD_SGIR MMIO (offset 0xF00 from GICD base).
//   GICv3: Write ICC_SGI1R_EL1 system register via MSR.
// The correct path is selected at runtime by checking a global flag.

static SHOOTDOWN_ADDR: AtomicU64 = AtomicU64::new(0);
static SHOOTDOWN_PENDING: AtomicUsize = AtomicUsize::new(0);

/// Set to `true` if the platform has a GICv3 (use MSR instead of MMIO).
static GICV3_PRESENT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

pub fn set_gicv3(present: bool) {
    GICV3_PRESENT.store(present, Ordering::Relaxed);
}

/// Send SGI 15 to all-but-self.
/// GICv2 path: write GICD_SGIR with TargetListFilter = 0b01 (all but self).
/// GICv3 path: write ICC_SGI1R_EL1 with IRM=1 (route to all but self).
unsafe fn send_sgi_tlb_shootdown() {
    if GICV3_PRESENT.load(Ordering::Relaxed) {
        // GICv3: ICC_SGI1R_EL1
        // Bit[40]:   IRM   = 1 (all cores, excluding self)
        // Bit[27:24]: INTID = SGI_TLB_SHOOTDOWN (15)
        let val: u64 = ICC_SGI1R_IRM_BIT | ((SGI_TLB_SHOOTDOWN as u64) << ICC_SGI1R_INTID_SHIFT);
        unsafe {
            core::arch::asm!("msr icc_sgi1r_el1, {}", in(reg) val);
            core::arch::asm!("isb");
        }
    } else {
        // GICv2: GICD_SGIR MMIO
        let gicd_base = crate::hal::aarch64::gic::GIC.lock().gicd_base_addr();
        if gicd_base == 0 {
            return;
        } // GIC not yet initialised
          // GICD_SGIR: TargetListFilter[25:24]=01, SGIINTID[3:0]=SGI_ID
        let sgir: u32 = GICD_SGIR_TARGET_ALL_BUT_SELF | (SGI_TLB_SHOOTDOWN as u32);
        unsafe {
            core::ptr::write_volatile(
                (gicd_base + crate::hal::aarch64::gic::GICD_SGIR_OFFSET) as *mut u32,
                sgir,
            )
        };
    }
}

/// Broadcast a TLB shootdown to all other cores and wait for completion.
///
/// The caller's TLB is flushed inline.  Remote cores flush in their SGI handler
/// and decrement `SHOOTDOWN_PENDING`.  We spin until it reaches 0.
///
/// # Ordering
/// `SHOOTDOWN_ADDR` is stored with Release before `SHOOTDOWN_PENDING` is set
/// with Release.  The handler loads with Acquire so it always sees the correct
/// address.
pub fn broadcast_tlb_shootdown(addr: u64) {
    let n = cpu_count();

    // Always flush locally first.
    unsafe {
        tlbi_vaae1is(addr);
    }

    if n <= 1 {
        return;
    } // uniprocessor: nothing else to do

    // Publish address before incrementing the counter (Release).
    SHOOTDOWN_ADDR.store(addr, Ordering::Release);
    SHOOTDOWN_PENDING.store(n - 1, Ordering::Release);

    // Issue the SGI.
    unsafe {
        send_sgi_tlb_shootdown();
    }

    // Spin-wait with a bounded timeout (avoids wedging if an AP is stuck).
    if let Some(left) = wait_for_pending_acks(
        &SHOOTDOWN_PENDING,
        AARCH64_TLB_SHOOTDOWN_TIMEOUT_SPINS.max(1),
    ) {
        TLB_SHOOTDOWN_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
        crate::klog_warn!("TLB shootdown timeout: {} AP(s) did not respond", left);
        SHOOTDOWN_PENDING.store(0, Ordering::Release);
    }
}

/// Perform the TLBI + barriers inline.
/// `addr` is a virtual address; the instruction takes bits [55:12] as the PFN.
unsafe fn tlbi_vaae1is(addr: u64) {
    unsafe {
        core::arch::asm!(
            "dsb ishst",              // ensure all preceding stores are visible
            "tlbi vaae1is, {0}",      // invalidate by VA, all ASID, inner-shareable
            "dsb ish",                // wait for the invalidation to complete
            "isb",                    // flush the pipeline
            in(reg) addr >> 12,
        )
    };
}

/// SGI 15 interrupt handler — called on each remote core.
///
/// Must be invoked by the exception handler after the SGI has been
/// acknowledged via GICC_IAR / ICC_IAR1_EL1.
pub fn handle_tlb_shootdown_ipi() {
    if SHOOTDOWN_PENDING.load(Ordering::Acquire) == 0 {
        return;
    }
    let addr = SHOOTDOWN_ADDR.load(Ordering::Acquire);
    unsafe {
        tlbi_vaae1is(addr);
    }
    // Signal the initiator that we are done.
    acknowledge_pending(&SHOOTDOWN_PENDING);
}

// ── PSCI helpers ─────────────────────────────────────────────────────────────

/// PSCI function IDs (SMC32 / HVC32 convention).
#[allow(dead_code)]
const PSCI_CPU_ON_32: u32 = 0x8400_0003;
const PSCI_CPU_ON_64: u64 = 0xC400_0003;

/// Call PSCI `CPU_ON` via HVC (Hypervisor Call).
/// `mpidr` – target CPU affinity, `entry` – physical entry address.
unsafe fn psci_cpu_on_hvc(mpidr: u64, entry: usize) -> i32 {
    let ret: i64;
    unsafe {
        core::arch::asm!(
            "hvc #0",
            inlateout("x0") PSCI_CPU_ON_64 as i64 => ret,
            in("x1") mpidr,
            in("x2") entry as u64,
            in("x3") 0u64,
            options(nomem, nostack)
        )
    };
    ret as i32
}

/// Call PSCI `CPU_ON` via SMC (Secure Monitor Call — bare-metal / TrustZone).
unsafe fn psci_cpu_on_smc(mpidr: u64, entry: usize) -> i32 {
    let ret: i64;
    unsafe {
        core::arch::asm!(
            "smc #0",
            inlateout("x0") PSCI_CPU_ON_64 as i64 => ret,
            in("x1") mpidr,
            in("x2") entry as u64,
            in("x3") 0u64,
            options(nomem, nostack)
        )
    };
    ret as i32
}

// ── AP boot flag ─────────────────────────────────────────────────────────────

/// Set to `1` by the AP once it has completed its own init.
static AP_READY: AtomicU32 = AtomicU32::new(0);
static PSCI_HVC_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static PSCI_HVC_SUCCESS: AtomicU64 = AtomicU64::new(0);
static PSCI_SMC_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static PSCI_SMC_SUCCESS: AtomicU64 = AtomicU64::new(0);
static PSCI_BOOT_FAILURES: AtomicU64 = AtomicU64::new(0);
static PSCI_BOOT_TIMEOUTS: AtomicU64 = AtomicU64::new(0);
static TLB_SHOOTDOWN_TIMEOUTS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct PsciBootStats {
    pub hvc_attempts: u64,
    pub hvc_success: u64,
    pub smc_attempts: u64,
    pub smc_success: u64,
    pub boot_failures: u64,
    pub boot_timeouts: u64,
    pub aps_ready: u32,
}

pub fn boot_stats() -> PsciBootStats {
    PsciBootStats {
        hvc_attempts: PSCI_HVC_ATTEMPTS.load(Ordering::Relaxed),
        hvc_success: PSCI_HVC_SUCCESS.load(Ordering::Relaxed),
        smc_attempts: PSCI_SMC_ATTEMPTS.load(Ordering::Relaxed),
        smc_success: PSCI_SMC_SUCCESS.load(Ordering::Relaxed),
        boot_failures: PSCI_BOOT_FAILURES.load(Ordering::Relaxed),
        boot_timeouts: PSCI_BOOT_TIMEOUTS.load(Ordering::Relaxed),
        aps_ready: AP_READY.load(Ordering::Relaxed),
    }
}

pub fn wait_stats() -> SmpWaitStats {
    build_wait_stats(
        AARCH64_SMP_BOOT_TIMEOUT_SPINS.max(1) as usize,
        PSCI_BOOT_TIMEOUTS.load(Ordering::Relaxed),
        AARCH64_TLB_SHOOTDOWN_TIMEOUT_SPINS.max(1),
        TLB_SHOOTDOWN_TIMEOUTS.load(Ordering::Relaxed),
    )
}

// ── AP entry point ────────────────────────────────────────────────────────────

/// Entry point jumped to by every Application Processor.
/// Must be `extern "C"` and at a known physical address.
#[unsafe(no_mangle)]
pub extern "C" fn aarch64_ap_entry() -> ! {
    // Read our MPIDR to derive a logical CPU ID.
    let mpidr: u64;
    unsafe {
        core::arch::asm!("mrs {}, mpidr_el1", out(reg) mpidr);
    }
    let cpu_index = (mpidr & MPIDR_AFF0_MASK) as usize; // Aff0 field
    let cpu_id = CpuId(cpu_index);

    // Allocate and install a per-CPU struct.
    let cpu_local = Box::leak(Box::new(CpuLocal {
        cpu_id,
        #[cfg(feature = "ring_protection")]
        scratch: 0,
        #[cfg(feature = "ring_protection")]
        kernel_stack_top: core::sync::atomic::AtomicUsize::new(allocate_kernel_stack_top()),
        current_task: core::sync::atomic::AtomicUsize::new(0),
        heartbeat_tick: core::sync::atomic::AtomicU64::new(0),
        idle_stack_pointer: core::sync::atomic::AtomicUsize::new(0),
        scheduler: crate::kernel::sync::IrqSafeMutex::new(
            crate::modules::selector::ActiveScheduler::new(),
        ),
    }));

    unsafe {
        cpu_local.init();
    }
    register_cpu(cpu_local);

    // Enable the generic timer interrupt (PPI 27).
    super::timer::GenericTimer::enable();

    // Signal BSP that we are alive.
    AP_READY.fetch_add(1, Ordering::Release);

    crate::klog_info!("AArch64 AP {} online (mpidr={:#x})", cpu_index, mpidr);

    // Enable interrupts and drop into the idle loop.
    unsafe {
        core::arch::asm!("msr daifclr, #{mask}", mask = const DAIFCLR_UNMASK_ALL);
    }
    loop {
        crate::kernel::idle_once();
    }
}

// ── BSP SMP initialisation ────────────────────────────────────────────────────

lazy_static::lazy_static! {
    /// Known MPIDR values to try waking.
    /// Default values cover QEMU `virt` and RPi4.
    /// Additional cores are added via `register_known_mpidr` (e.g. from DTB/ACPI parsing).
    pub static ref KNOWN_MPIDRS: IrqSafeMutex<Vec<u64>> =
        IrqSafeMutex::new(AARCH64_SMP_KNOWN_MPIDRS.to_vec());
}

/// Register a target CPU for waking.
pub fn register_known_mpidr(mpidr: u64) {
    let mut list = KNOWN_MPIDRS.lock();
    if !list.contains(&mpidr) {
        list.push(mpidr);
    }
}

pub fn init() {
    let entry = aarch64_ap_entry as *const () as usize;
    let mpidrs = KNOWN_MPIDRS.lock().clone();

    // Attempt HVC first; fall back to SMC.
    for &mpidr in &mpidrs {
        PSCI_HVC_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        let ret = unsafe { psci_cpu_on_hvc(mpidr, entry) };
        if ret == 0 {
            PSCI_HVC_SUCCESS.fetch_add(1, Ordering::Relaxed);
            crate::klog_info!("AArch64 SMP: woke CPU mpidr={:#x} via HVC", mpidr);
        } else {
            PSCI_SMC_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
            let ret2 = unsafe { psci_cpu_on_smc(mpidr, entry) };
            if ret2 == 0 {
                PSCI_SMC_SUCCESS.fetch_add(1, Ordering::Relaxed);
                crate::klog_info!("AArch64 SMP: woke CPU mpidr={:#x} via SMC", mpidr);
            } else {
                PSCI_BOOT_FAILURES.fetch_add(1, Ordering::Relaxed);
                crate::klog_warn!(
                    "AArch64 SMP: CPU mpidr={:#x} failed (hvc={} smc={})",
                    mpidr,
                    ret,
                    ret2
                );
            }
        }
    }

    // Wait for APs to finish init (with a spin timeout).
    let expected = mpidrs.len() as u32;
    let timed_out = !wait_for_ready_count(
        &AP_READY,
        expected,
        AARCH64_SMP_BOOT_TIMEOUT_SPINS.max(1) as usize,
    );
    if timed_out {
        PSCI_BOOT_TIMEOUTS.fetch_add(1, Ordering::Relaxed);
    }

    let online = AP_READY.load(Ordering::Relaxed);
    crate::klog_info!("AArch64 SMP: {}/{} APs online", online, expected);
}
