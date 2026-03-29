use crate::interfaces::Dispatcher;
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// Vectored Exception/Interrupt Dispatcher.
/// Allows dynamic registration of multiple handlers for specific IRQs.
/// Thread-safe via IrqSafeMutex.

type HandlerFn = fn(u8);

#[derive(Clone, Copy)]
struct HandlerRegistration {
    handler: fn(u8, usize) -> bool,
    ctx: usize,
}

const SHARED_IRQ_STORM_FANOUT_THRESHOLD: u64 = 8;
const IRQ_STORM_WINDOW_DISPATCHES: u64 = 1024;
const IRQ_STORM_PER_WINDOW_LIMIT: u64 = 256;

static DISPATCH_REGISTER_CALLS: AtomicU64 = AtomicU64::new(0);
static DISPATCH_CALLS: AtomicU64 = AtomicU64::new(0);
static DISPATCH_HANDLED_CALLS: AtomicU64 = AtomicU64::new(0);
static DISPATCH_DEFAULT_HITS: AtomicU64 = AtomicU64::new(0);
static DISPATCH_HANDLER_INVOCATIONS: AtomicU64 = AtomicU64::new(0);
static DISPATCH_MAX_FANOUT: AtomicU64 = AtomicU64::new(0);
static DISPATCH_STORM_HINTS: AtomicU64 = AtomicU64::new(0);
static DISPATCH_THROTTLED: AtomicU64 = AtomicU64::new(0);
static DISPATCH_WINDOW_RESETS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct VectoredDispatchStats {
    pub register_calls: u64,
    pub dispatch_calls: u64,
    pub handled_calls: u64,
    pub default_hits: u64,
    pub handler_invocations: u64,
    pub max_fanout: u64,
    pub storm_hints: u64,
    pub throttled: u64,
    pub window_resets: u64,
}

pub fn stats() -> VectoredDispatchStats {
    VectoredDispatchStats {
        register_calls: DISPATCH_REGISTER_CALLS.load(Ordering::Relaxed),
        dispatch_calls: DISPATCH_CALLS.load(Ordering::Relaxed),
        handled_calls: DISPATCH_HANDLED_CALLS.load(Ordering::Relaxed),
        default_hits: DISPATCH_DEFAULT_HITS.load(Ordering::Relaxed),
        handler_invocations: DISPATCH_HANDLER_INVOCATIONS.load(Ordering::Relaxed),
        max_fanout: DISPATCH_MAX_FANOUT.load(Ordering::Relaxed),
        storm_hints: DISPATCH_STORM_HINTS.load(Ordering::Relaxed),
        throttled: DISPATCH_THROTTLED.load(Ordering::Relaxed),
        window_resets: DISPATCH_WINDOW_RESETS.load(Ordering::Relaxed),
    }
}

fn update_max_fanout(fanout: u64) {
    let mut current = DISPATCH_MAX_FANOUT.load(Ordering::Relaxed);
    while fanout > current {
        match DISPATCH_MAX_FANOUT.compare_exchange_weak(
            current,
            fanout,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
}

pub struct VectoredDispatcher {
    handlers: IrqSafeMutex<BTreeMap<u8, Vec<HandlerRegistration>>>,
    default_handler: IrqSafeMutex<Option<HandlerFn>>,
    storm_window_counts: IrqSafeMutex<BTreeMap<u8, u64>>,
    storm_window_index: AtomicU64,
}

impl Default for VectoredDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl VectoredDispatcher {
    pub const fn new() -> Self {
        Self {
            handlers: IrqSafeMutex::new(BTreeMap::new()),
            default_handler: IrqSafeMutex::new(None),
            storm_window_counts: IrqSafeMutex::new(BTreeMap::new()),
            storm_window_index: AtomicU64::new(0),
        }
    }

    /// Register a catch-all handler for unhandled IRQs.
    pub fn set_default_handler(&self, handler: HandlerFn) {
        let mut def = self.default_handler.lock();
        *def = Some(handler);
    }

    fn should_throttle_irq(&self, irq: u8, dispatch_sequence: u64) -> bool {
        let window = dispatch_sequence / IRQ_STORM_WINDOW_DISPATCHES;
        let prev_window = self.storm_window_index.load(Ordering::Relaxed);
        if prev_window != window {
            self.storm_window_index.store(window, Ordering::Relaxed);
            self.storm_window_counts.lock().clear();
            DISPATCH_WINDOW_RESETS.fetch_add(1, Ordering::Relaxed);
        }

        let mut counts = self.storm_window_counts.lock();
        let entry = counts.entry(irq).or_insert(0);
        *entry = entry.saturating_add(1);
        *entry > IRQ_STORM_PER_WINDOW_LIMIT
    }
}

impl Dispatcher for VectoredDispatcher {
    fn register_handler(&self, irq: u8, handler: fn(u8)) {
        DISPATCH_REGISTER_CALLS.fetch_add(1, Ordering::Relaxed);
        let mut map = self.handlers.lock();
        let entry = map.entry(irq).or_insert_with(Vec::new);

        // Wrap legacy handler
        fn legacy_wrapper(irq: u8, ctx: usize) -> bool {
            let func: fn(u8) = unsafe { core::mem::transmute(ctx as *const ()) };
            func(irq);
            false // Legacy handlers don't return bool, we scan all of them.
        }

        entry.push(HandlerRegistration {
            handler: legacy_wrapper,
            ctx: handler as usize,
        });
        update_max_fanout(entry.len() as u64);
    }

    fn register_handler_with_ctx(&self, irq: u8, handler: fn(u8, usize) -> bool, ctx: usize) {
        DISPATCH_REGISTER_CALLS.fetch_add(1, Ordering::Relaxed);
        let mut map = self.handlers.lock();
        let entry = map.entry(irq).or_insert_with(Vec::new);
        entry.push(HandlerRegistration { handler, ctx });
        update_max_fanout(entry.len() as u64);
    }

    fn dispatch(&self, irq: u8) {
        let dispatch_sequence = DISPATCH_CALLS.fetch_add(1, Ordering::Relaxed) + 1;
        if self.should_throttle_irq(irq, dispatch_sequence) {
            DISPATCH_THROTTLED.fetch_add(1, Ordering::Relaxed);
            DISPATCH_STORM_HINTS.fetch_add(1, Ordering::Relaxed);
            return;
        }
        // Lock 1: Default Handler (check first?)
        // Actually, check specific handlers first.
        let map = self.handlers.lock();

        if let Some(handlers) = map.get(&irq) {
            if !handlers.is_empty() {
                let fanout = handlers.len() as u64;
                update_max_fanout(fanout);
                DISPATCH_HANDLED_CALLS.fetch_add(1, Ordering::Relaxed);
                DISPATCH_HANDLER_INVOCATIONS.fetch_add(fanout, Ordering::Relaxed);
                if fanout >= SHARED_IRQ_STORM_FANOUT_THRESHOLD {
                    DISPATCH_STORM_HINTS.fetch_add(1, Ordering::Relaxed);
                }
                // Execute all registered handlers in order
                for reg in handlers {
                    if (reg.handler)(irq, reg.ctx) {
                        break; // Handled, no need to fan-out further
                    }
                }
                let _ = crate::modules::dispatcher::upcall::mark_global_delivered_for_irq(irq);
                return;
            }
        }

        // Lock 2: Default Handler
        // Drop map lock first? No, IrqSafeMutex is reentrant on different locks on same CPU?
        // No, it's a spinlock. It disbles interrupts.
        // It is safe to take two different locks.

        // HOWEVER, to be safe and reduce contention, drop map first.
        drop(map);

        // Fallback
        if let Some(default) = *self.default_handler.lock() {
            DISPATCH_DEFAULT_HITS.fetch_add(1, Ordering::Relaxed);
            default(irq);
            let _ = crate::modules::dispatcher::upcall::mark_global_delivered_for_irq(irq);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use core::sync::atomic::{AtomicU64, Ordering};

    static HANDLER_HITS: AtomicU64 = AtomicU64::new(0);
    static DEFAULT_HITS: AtomicU64 = AtomicU64::new(0);

    fn test_handler(_irq: u8) {
        HANDLER_HITS.fetch_add(1, Ordering::Relaxed);
    }

    fn default_handler(_irq: u8) {
        DEFAULT_HITS.fetch_add(1, Ordering::Relaxed);
    }

    #[test_case]
    fn vectored_dispatch_invokes_default_handler_when_unhandled() {
        DEFAULT_HITS.store(0, Ordering::Relaxed);
        let disp = VectoredDispatcher::new();
        disp.set_default_handler(default_handler);

        disp.dispatch(250);

        assert_eq!(DEFAULT_HITS.load(Ordering::Relaxed), 1);
    }

    #[test_case]
    fn vectored_dispatch_throttles_irq_storms() {
        HANDLER_HITS.store(0, Ordering::Relaxed);
        let disp = VectoredDispatcher::new();
        disp.register_handler(33, test_handler);

        for _ in 0..300 {
            disp.dispatch(33);
        }

        let hits = HANDLER_HITS.load(Ordering::Relaxed);
        let s = stats();
        assert!(hits <= IRQ_STORM_PER_WINDOW_LIMIT);
        assert!(s.throttled >= 1);
    }
}
