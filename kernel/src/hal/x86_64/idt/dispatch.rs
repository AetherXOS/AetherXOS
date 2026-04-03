#[cfg(feature = "dispatcher")]
use crate::modules::dispatcher::selector::ActiveDispatcher;
#[cfg(target_os = "none")]
use crate::kernel::syscalls::syscalls_consts::x86;
#[cfg(target_os = "none")]
use crate::hal::common::irq_catalog::IrqLineKind;
#[cfg(target_os = "none")]
use crate::hal::common::irq_trace;
#[cfg(target_os = "none")]
use core::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "dispatcher")]
use spin::Mutex;

#[cfg(feature = "dispatcher")]
static DISPATCHER: Mutex<Option<ActiveDispatcher>> = Mutex::new(None);

#[cfg(target_os = "none")]
static IRQ_TOTAL: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "none")]
static IRQ_TIMER: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "none")]
static IRQ_NON_TIMER: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "none")]
static IRQ_DROPPED: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "none")]
static IRQ_DISPATCH_ATTEMPTED: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "none")]
static IRQ_DISPATCH_HANDLED: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "none")]
#[derive(Debug, Clone, Copy)]
pub struct IrqDispatchMetrics {
    pub total: u64,
    pub timer: u64,
    pub non_timer: u64,
    pub dropped: u64,
    pub dispatch_attempted: u64,
    pub dispatch_handled: u64,
}

#[cfg(feature = "dispatcher")]
pub fn init_dispatcher(d: ActiveDispatcher) {
    *DISPATCHER.lock() = Some(d);
}

#[cfg(not(feature = "dispatcher"))]
pub fn init_dispatcher(_d: ()) {}

#[cfg(target_os = "none")]
#[inline(always)]
pub fn irq_dispatch_metrics() -> IrqDispatchMetrics {
    IrqDispatchMetrics {
        total: IRQ_TOTAL.load(Ordering::Relaxed),
        timer: IRQ_TIMER.load(Ordering::Relaxed),
        non_timer: IRQ_NON_TIMER.load(Ordering::Relaxed),
        dropped: IRQ_DROPPED.load(Ordering::Relaxed),
        dispatch_attempted: IRQ_DISPATCH_ATTEMPTED.load(Ordering::Relaxed),
        dispatch_handled: IRQ_DISPATCH_HANDLED.load(Ordering::Relaxed),
    }
}

#[cfg(target_os = "none")]
#[inline(always)]
pub(super) fn try_dispatch_vector(vector: u8) -> bool {
    #[cfg(feature = "dispatcher")]
    {
        use crate::interfaces::Dispatcher;
        if let Some(d) = DISPATCHER.lock().as_ref() {
            d.dispatch(vector);
            return true;
        }
    }

    false
}

#[cfg(target_os = "none")]
#[inline(always)]
pub(super) fn dispatch_irq_vector(vector: u8) {
    let descriptor = super::metadata::irq_descriptor(vector);
    let allow_dispatch = crate::kernel::interrupt_guard::on_irq(vector);
    let is_timer = descriptor.kind == IrqLineKind::Timer;

    IRQ_TOTAL.fetch_add(1, Ordering::Relaxed);
    if is_timer {
        IRQ_TIMER.fetch_add(1, Ordering::Relaxed);
    } else {
        IRQ_NON_TIMER.fetch_add(1, Ordering::Relaxed);
    }

    irq_trace::trace_dispatched(
        "x86_64",
        vector as u64,
        descriptor.label,
        crate::generated_consts::CORE_ENABLE_IRQ_TRACE && !is_timer,
    );

    if allow_dispatch {
        IRQ_DISPATCH_ATTEMPTED.fetch_add(1, Ordering::Relaxed);
        if try_dispatch_vector(vector) {
            IRQ_DISPATCH_HANDLED.fetch_add(1, Ordering::Relaxed);
        }
    } else {
        IRQ_DROPPED.fetch_add(1, Ordering::Relaxed);
        irq_trace::trace_dropped_by_storm(
            "x86_64",
            vector as u64,
            descriptor.label,
            crate::generated_consts::CORE_ENABLE_IRQ_TRACE,
        );
    }

    // Safety: local APIC EOI must be issued after IRQ handling on this CPU.
    unsafe {
        crate::hal::apic::eoi();
    }
}
