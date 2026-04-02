use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::generated_consts::{
    AARCH64_EXCEPTION_KILL_USER_ASYNC, AARCH64_EXCEPTION_KILL_USER_SYNC,
    AARCH64_EXCEPTION_PANIC_ON_KERNEL_ASYNC, AARCH64_EXCEPTION_PANIC_ON_KERNEL_SYNC,
    AARCH64_GIC_CPU_PRIORITY_MASK, AARCH64_IRQ_PER_LINE_LOG_EVERY,
    AARCH64_IRQ_PER_LINE_STORM_THRESHOLD, AARCH64_IRQ_RATE_TRACK_LIMIT,
    AARCH64_IRQ_STORM_LOG_EVERY, AARCH64_IRQ_STORM_THRESHOLD, AARCH64_IRQ_STORM_WINDOW_TICKS,
    AARCH64_TIMER_JITTER_TOLERANCE_TICKS,
};
use crate::hal::common::irq::{
    abs_diff_u64, hottest_counter_index, reset_window, storm_decision, tracked_limit,
};
use crate::interfaces::HardwareAbstraction;

#[path = "exception/vector_table.rs"]
mod vector_table;
#[path = "exception/fault_policy.rs"]
mod fault_policy;
use fault_policy::{handle_kernel_fault, handle_user_fault, is_lower_el_exception};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ExceptionFrame {
    pub registers: [u64; 31],
    pub spsr: u64,
    pub elr: u64,
    pub sp_el0: u64,
}

static SYNC_EXCEPTIONS: AtomicU64 = AtomicU64::new(0);
static FIQ_EXCEPTIONS: AtomicU64 = AtomicU64::new(0);
static SERROR_EXCEPTIONS: AtomicU64 = AtomicU64::new(0);
static USER_ABORT_EXCEPTIONS: AtomicU64 = AtomicU64::new(0);
static KERNEL_ABORT_EXCEPTIONS: AtomicU64 = AtomicU64::new(0);
static USER_FATAL_SYNC_EXCEPTIONS: AtomicU64 = AtomicU64::new(0);
static USER_FATAL_ASYNC_EXCEPTIONS: AtomicU64 = AtomicU64::new(0);
static KERNEL_FATAL_ASYNC_EXCEPTIONS: AtomicU64 = AtomicU64::new(0);
static IRQ_TOTAL_EXCEPTIONS: AtomicU64 = AtomicU64::new(0);
static IRQ_SPURIOUS_EXCEPTIONS: AtomicU64 = AtomicU64::new(0);
static IRQ_STORM_WINDOWS: AtomicU64 = AtomicU64::new(0);
static IRQ_STORM_SUPPRESSED_LOGS: AtomicU64 = AtomicU64::new(0);
static TIMER_IRQ_COUNT: AtomicU64 = AtomicU64::new(0);
static TIMER_IRQ_JITTER_EVENTS: AtomicU64 = AtomicU64::new(0);
static IRQ_WINDOW_START_COUNTER: AtomicU64 = AtomicU64::new(0);
static IRQ_WINDOW_EVENT_COUNT: AtomicU64 = AtomicU64::new(0);
static TIMER_LAST_IRQ_COUNTER: AtomicU64 = AtomicU64::new(0);
const MAX_TRACKED_IRQS: usize = 256;

#[derive(Debug, Clone)]
struct IrqRateTracker {
    total: [u64; MAX_TRACKED_IRQS],
    window_start: [u64; MAX_TRACKED_IRQS],
    window_count: [u64; MAX_TRACKED_IRQS],
    storm_events: [u64; MAX_TRACKED_IRQS],
    suppressed_logs: [u64; MAX_TRACKED_IRQS],
}

impl IrqRateTracker {
    const fn new() -> Self {
        Self {
            total: [0; MAX_TRACKED_IRQS],
            window_start: [0; MAX_TRACKED_IRQS],
            window_count: [0; MAX_TRACKED_IRQS],
            storm_events: [0; MAX_TRACKED_IRQS],
            suppressed_logs: [0; MAX_TRACKED_IRQS],
        }
    }
}

static IRQ_RATE_TRACKER: Mutex<IrqRateTracker> = Mutex::new(IrqRateTracker::new());

const ESR_EC_UNKNOWN: u64 = 0b000000;
const ESR_EC_WFI_WFE: u64 = 0b000001;
const ESR_EC_FP_TRAP: u64 = 0b000111;
const ESR_EC_ILLEGAL_STATE: u64 = 0b001110;
const ESR_EC_SVC64: u64 = 0b010101;
const ESR_EC_IABORT_LOWER_EL: u64 = 0b100000;
const ESR_EC_IABORT_CURRENT_EL: u64 = 0b100001;
const ESR_EC_PC_ALIGNMENT: u64 = 0b100010;
const ESR_EC_DABORT_LOWER_EL: u64 = 0b100100;
const ESR_EC_DABORT_CURRENT_EL: u64 = 0b100101;
const ESR_EC_SP_ALIGNMENT: u64 = 0b100110;
const ESR_EC_BREAKPOINT_LOWER_EL: u64 = 0b110000;
const ESR_EC_BREAKPOINT_CURRENT_EL: u64 = 0b110001;
const ESR_EC_BRK: u64 = 0b111100;

#[derive(Debug, Clone, Copy)]
pub struct ExceptionStats {
    pub sync_exceptions: u64,
    pub fiq_exceptions: u64,
    pub serror_exceptions: u64,
    pub user_abort_exceptions: u64,
    pub kernel_abort_exceptions: u64,
    pub user_fatal_sync_exceptions: u64,
    pub user_fatal_async_exceptions: u64,
    pub kernel_fatal_async_exceptions: u64,
    pub irq_total_exceptions: u64,
    pub irq_spurious_exceptions: u64,
    pub irq_storm_windows: u64,
    pub irq_storm_suppressed_logs: u64,
    pub timer_irq_count: u64,
    pub timer_irq_jitter_events: u64,
    pub irq_track_limit: usize,
    pub hottest_irq_line: usize,
    pub hottest_irq_total: u64,
    pub hottest_irq_storm_events: u64,
    pub hottest_irq_suppressed_logs: u64,
    pub gic_cpu_priority_mask: u32,
}

#[inline(always)]
pub fn stats() -> ExceptionStats {
    let tracked = tracked_limit(AARCH64_IRQ_RATE_TRACK_LIMIT, MAX_TRACKED_IRQS);
    let (
        hottest_irq_line,
        hottest_irq_total,
        hottest_irq_storm_events,
        hottest_irq_suppressed_logs,
    ) = {
        let tracker = IRQ_RATE_TRACKER.lock();
        let best_idx = hottest_counter_index(&tracker.total[..tracked]);
        (
            best_idx,
            tracker.total[best_idx],
            tracker.storm_events[best_idx],
            tracker.suppressed_logs[best_idx],
        )
    };

    ExceptionStats {
        sync_exceptions: SYNC_EXCEPTIONS.load(Ordering::Relaxed),
        fiq_exceptions: FIQ_EXCEPTIONS.load(Ordering::Relaxed),
        serror_exceptions: SERROR_EXCEPTIONS.load(Ordering::Relaxed),
        user_abort_exceptions: USER_ABORT_EXCEPTIONS.load(Ordering::Relaxed),
        kernel_abort_exceptions: KERNEL_ABORT_EXCEPTIONS.load(Ordering::Relaxed),
        user_fatal_sync_exceptions: USER_FATAL_SYNC_EXCEPTIONS.load(Ordering::Relaxed),
        user_fatal_async_exceptions: USER_FATAL_ASYNC_EXCEPTIONS.load(Ordering::Relaxed),
        kernel_fatal_async_exceptions: KERNEL_FATAL_ASYNC_EXCEPTIONS.load(Ordering::Relaxed),
        irq_total_exceptions: IRQ_TOTAL_EXCEPTIONS.load(Ordering::Relaxed),
        irq_spurious_exceptions: IRQ_SPURIOUS_EXCEPTIONS.load(Ordering::Relaxed),
        irq_storm_windows: IRQ_STORM_WINDOWS.load(Ordering::Relaxed),
        irq_storm_suppressed_logs: IRQ_STORM_SUPPRESSED_LOGS.load(Ordering::Relaxed),
        timer_irq_count: TIMER_IRQ_COUNT.load(Ordering::Relaxed),
        timer_irq_jitter_events: TIMER_IRQ_JITTER_EVENTS.load(Ordering::Relaxed),
        irq_track_limit: tracked,
        hottest_irq_line,
        hottest_irq_total,
        hottest_irq_storm_events,
        hottest_irq_suppressed_logs,
        gic_cpu_priority_mask: AARCH64_GIC_CPU_PRIORITY_MASK.min(0xFF),
    }
}

#[no_mangle]
pub extern "C" fn handle_sync(frame: &mut ExceptionFrame) {
    SYNC_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);

    let esr: u64;
    let far: u64;
    unsafe {
        core::arch::asm!("mrs {}, esr_el1", out(reg) esr);
        core::arch::asm!("mrs {}, far_el1", out(reg) far);
    }

    use crate::kernel::bit_utils::io::aarch64_sys as esr_bits;
    let ec = esr_bits::ESR_EC.read(esr as u32) as u64;
    let iss = esr_bits::ESR_ISS.read(esr as u32) as u64;

    let ec_str = match ec {
        ESR_EC_UNKNOWN => "Unknown Reason",
        ESR_EC_WFI_WFE => "WFI/WFE Trap",
        ESR_EC_FP_TRAP => "SVE/SIMD/FP Trap",
        ESR_EC_ILLEGAL_STATE => "Illegal Execution State",
        ESR_EC_SVC64 => "SVC in 64-bit state",
        ESR_EC_IABORT_LOWER_EL => "Instruction Abort (Lower EL)",
        ESR_EC_IABORT_CURRENT_EL => "Instruction Abort (Current EL)",
        ESR_EC_PC_ALIGNMENT => "PC Alignment Fault",
        ESR_EC_DABORT_LOWER_EL => "Data Abort (Lower EL)",
        ESR_EC_DABORT_CURRENT_EL => "Data Abort (Current EL)",
        ESR_EC_SP_ALIGNMENT => "SP Alignment Fault",
        ESR_EC_BREAKPOINT_LOWER_EL => "Breakpoint (Lower EL)",
        ESR_EC_BREAKPOINT_CURRENT_EL => "Breakpoint (Current EL)",
        ESR_EC_BRK => "BRK Instruction",
        _ => "Other/Unknown",
    };

    crate::klog_warn!(
        "Synchronous Exception: {} (EC: {:#08b}, ISS: {:#x})",
        ec_str,
        ec,
        iss
    );
    crate::klog_warn!(
        "FAR_EL1: {:#x}, ELR_EL1: {:#x}, SPSR_EL1: {:#x}",
        far,
        frame.elr,
        frame.spsr
    );
    if crate::config::KernelConfig::is_advanced_debug_enabled() {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                (frame as *const ExceptionFrame).cast::<u8>(),
                core::mem::size_of::<ExceptionFrame>(),
            )
        };
        crate::hal::aarch64::serial::write_dump_bytes("aarch64.sync.frame", bytes);
    }
    crate::kernel::debug_trace::record_register_snapshot(
        "aarch64.sync",
        frame.elr,
        frame.sp_el0,
        far,
        esr,
    );

    if ec == esr_bits::EC_DABORT_CUR as u64 || ec == esr_bits::EC_DABORT_LOWER as u64 {
        if is_lower_el_exception(frame) {
            #[cfg(feature = "paging_enable")]
            if crate::kernel::vmm::handle_user_page_fault(far).is_ok() {
                return;
            }
            USER_ABORT_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
            handle_user_fault("data-abort", ec, far, frame.elr, false);
        }
        KERNEL_ABORT_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
        handle_kernel_fault(
            "data-abort",
            ec,
            far,
            frame.elr,
            AARCH64_EXCEPTION_PANIC_ON_KERNEL_SYNC,
        );
    } else if ec == esr_bits::EC_IABORT_CUR as u64 || ec == esr_bits::EC_IABORT_LOWER as u64 {
        if is_lower_el_exception(frame) {
            #[cfg(feature = "paging_enable")]
            if crate::kernel::vmm::handle_user_page_fault(far).is_ok() {
                return;
            }
            USER_ABORT_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
            handle_user_fault("instruction-abort", ec, far, frame.elr, false);
        }
        KERNEL_ABORT_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
        handle_kernel_fault(
            "instruction-abort",
            ec,
            far,
            frame.elr,
            AARCH64_EXCEPTION_PANIC_ON_KERNEL_SYNC,
        );
    } else if ec == esr_bits::EC_SVC64 as u64 {
        crate::klog_warn!("SVC call: {}", iss);
        frame.elr += 4; // Skip SVC instruction
    } else {
        if is_lower_el_exception(frame) {
            USER_FATAL_SYNC_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
            handle_user_fault("unhandled-sync", ec, far, frame.elr, false);
        }
        handle_kernel_fault(
            "unhandled-sync",
            ec,
            far,
            frame.elr,
            AARCH64_EXCEPTION_PANIC_ON_KERNEL_SYNC,
        );
    }
}

#[no_mangle]
pub extern "C" fn handle_irq(_frame: &mut ExceptionFrame) {
    IRQ_TOTAL_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);

    let mut gic = crate::hal::aarch64::gic::GIC.lock();
    let iar = gic.read_iar();
    let irq_id = iar & 0x3FF;

    if irq_id < 1020 {
        // Genuine interrupt with global storm-window throttling.
        let now_counter = crate::hal::aarch64::timer::GenericTimer::counter();
        let window_ticks = AARCH64_IRQ_STORM_WINDOW_TICKS.max(1);
        let threshold = AARCH64_IRQ_STORM_THRESHOLD.max(1);
        let log_every = AARCH64_IRQ_STORM_LOG_EVERY.max(1);

        let start = IRQ_WINDOW_START_COUNTER.load(Ordering::Relaxed);
        if reset_window(start, now_counter, window_ticks) {
            IRQ_WINDOW_START_COUNTER.store(now_counter, Ordering::Relaxed);
            IRQ_WINDOW_EVENT_COUNT.store(0, Ordering::Relaxed);
        }

        let in_window = IRQ_WINDOW_EVENT_COUNT
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        let decision = storm_decision(in_window, threshold, log_every, true);
        if decision.first_storm_event {
            IRQ_STORM_WINDOWS.fetch_add(1, Ordering::Relaxed);
        } else if decision.suppressed_log {
            IRQ_STORM_SUPPRESSED_LOGS.fetch_add(1, Ordering::Relaxed);
        }
        if decision.should_log {
            crate::klog_debug!(
                "AArch64 IRQ {} Triggered window_events={} storm={}",
                irq_id,
                in_window,
                decision.in_storm
            );
        }

        let tracked_limit = tracked_limit(AARCH64_IRQ_RATE_TRACK_LIMIT, MAX_TRACKED_IRQS);
        if (irq_id as usize) < tracked_limit {
            let per_line_threshold = AARCH64_IRQ_PER_LINE_STORM_THRESHOLD.max(1);
            let per_line_log_every = AARCH64_IRQ_PER_LINE_LOG_EVERY.max(1);
            let idx = irq_id as usize;
            let mut tracker = IRQ_RATE_TRACKER.lock();
            tracker.total[idx] = tracker.total[idx].saturating_add(1);

            let start = tracker.window_start[idx];
            if reset_window(start, now_counter, window_ticks) {
                tracker.window_start[idx] = now_counter;
                tracker.window_count[idx] = 0;
            }
            tracker.window_count[idx] = tracker.window_count[idx].saturating_add(1);
            let line_count = tracker.window_count[idx];
            let line_decision =
                storm_decision(line_count, per_line_threshold, per_line_log_every, false);
            if line_decision.first_storm_event {
                tracker.storm_events[idx] = tracker.storm_events[idx].saturating_add(1);
            } else if line_decision.suppressed_log {
                tracker.suppressed_logs[idx] = tracker.suppressed_logs[idx].saturating_add(1);
            }
            drop(tracker);

            if line_decision.should_log {
                crate::klog_warn!(
                    "AArch64 IRQ line storm irq={} line_window_events={} threshold={}",
                    irq_id,
                    line_count,
                    per_line_threshold
                );
            }
        }

        // Timer interrupt
        if irq_id == 27 || irq_id == 30 {
            TIMER_IRQ_COUNT.fetch_add(1, Ordering::Relaxed);
            let last = TIMER_LAST_IRQ_COUNTER.swap(now_counter, Ordering::Relaxed);
            if last != 0 {
                let delta = now_counter.wrapping_sub(last);
                let target =
                    crate::hal::aarch64::timer::GenericTimer::last_programmed_ticks().max(1);
                let jitter = abs_diff_u64(delta, target);
                if jitter > AARCH64_TIMER_JITTER_TOLERANCE_TICKS {
                    TIMER_IRQ_JITTER_EVENTS.fetch_add(1, Ordering::Relaxed);
                }
            }
            crate::hal::aarch64::timer::GenericTimer::set_timer(
                crate::hal::aarch64::timer::GenericTimer::frequency() / 1000,
            );
        }

        // UART interrupt (QEMU virt UART SPI 1 = 32 + 1 = 33)
        if irq_id == 33 {
            let serial = crate::hal::aarch64::serial::SERIAL1.lock();
            let mut rx_buf = [0u8; 32];
            let n = serial.recv_burst(&mut rx_buf);
            if n > 0 {
                if let Some(tty) = crate::kernel::tty::GLOBAL_TTY_REGISTRY.lock().get(crate::kernel::tty::TtyId::new(0)) {
                    tty.push_input(&rx_buf[..n]);
                }
            }
            serial.clear_interrupts();
        }

        // Send EOI
        use crate::interfaces::InterruptController;
        unsafe { gic.end_of_interrupt(irq_id) };
    } else {
        IRQ_SPURIOUS_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
    }
}

#[no_mangle]
pub extern "C" fn handle_fiq(frame: &mut ExceptionFrame) {
    FIQ_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);

    let esr: u64;
    unsafe {
        core::arch::asm!("mrs {}, esr_el1", out(reg) esr);
    }
    let ec = (esr >> 26) & 0x3F;

    crate::klog_error!(
        "AArch64 FIQ: ec={:#x} elr={:#x} spsr={:#x}",
        ec,
        frame.elr,
        frame.spsr
    );

    if is_lower_el_exception(frame) {
        USER_FATAL_ASYNC_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
        handle_user_fault("fiq", ec, 0, frame.elr, true);
    }

    KERNEL_FATAL_ASYNC_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
    handle_kernel_fault(
        "fiq",
        ec,
        0,
        frame.elr,
        AARCH64_EXCEPTION_PANIC_ON_KERNEL_ASYNC,
    );
}

#[no_mangle]
pub extern "C" fn handle_serror(frame: &mut ExceptionFrame) {
    SERROR_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);

    let esr: u64;
    unsafe {
        core::arch::asm!("mrs {}, esr_el1", out(reg) esr);
    }
    let ec = (esr >> 26) & 0x3F;

    crate::klog_error!(
        "AArch64 SError: ec={:#x} elr={:#x} spsr={:#x}",
        ec,
        frame.elr,
        frame.spsr
    );

    if is_lower_el_exception(frame) {
        USER_FATAL_ASYNC_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
        handle_user_fault("serror", ec, 0, frame.elr, true);
    }

    KERNEL_FATAL_ASYNC_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
    handle_kernel_fault(
        "serror",
        ec,
        0,
        frame.elr,
        AARCH64_EXCEPTION_PANIC_ON_KERNEL_ASYNC,
    );
}

pub fn init() {
    unsafe {
        core::arch::asm!("msr vbar_el1, {}", in(reg) vector_table::table_ptr());
    }
}
