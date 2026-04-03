use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::generated_consts::{
    AARCH64_EXCEPTION_KILL_USER_ASYNC, AARCH64_EXCEPTION_KILL_USER_SYNC,
    AARCH64_GIC_CPU_PRIORITY_MASK,
};

#[path = "exception/vector_table.rs"]
mod vector_table;
#[path = "exception/fault_policy.rs"]
pub(super) mod fault_policy;
#[path = "exception/sync.rs"]
mod sync;
#[path = "exception/irq.rs"]
mod irq;

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

const GIC_CPU_PRIORITY_MASK_MAX: u32 = 0xFF;

static IRQ_RATE_TRACKER: Mutex<irq::IrqRateTracker> = Mutex::new(irq::IrqRateTracker::new());

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
    let (
        hottest_irq_line,
        hottest_irq_total,
        hottest_irq_storm_events,
        hottest_irq_suppressed_logs,
        irq_track_limit,
    ) = irq::hottest_irq_snapshot();

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
        irq_track_limit,
        hottest_irq_line,
        hottest_irq_total,
        hottest_irq_storm_events,
        hottest_irq_suppressed_logs,
        gic_cpu_priority_mask: AARCH64_GIC_CPU_PRIORITY_MASK.min(GIC_CPU_PRIORITY_MASK_MAX),
    }
}

pub fn init() {
    unsafe {
        core::arch::asm!("msr vbar_el1, {}", in(reg) vector_table::table_ptr());
    }
}
