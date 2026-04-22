use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

const IRQ_SLOTS: usize = 256;

static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);
static IRQ_EPOCH: AtomicU64 = AtomicU64::new(0);
static LAST_EPOCH: [AtomicU64; IRQ_SLOTS] = [const { AtomicU64::new(0) }; IRQ_SLOTS];
static IRQ_COUNT: [AtomicU32; IRQ_SLOTS] = [const { AtomicU32::new(0) }; IRQ_SLOTS];
static IRQ_DROPPED: [AtomicU64; IRQ_SLOTS] = [const { AtomicU64::new(0) }; IRQ_SLOTS];

#[inline(always)]
pub fn on_irq(vector: u8) -> bool {
    if !crate::generated_consts::CORE_ENABLE_INTERRUPT_STORM_PROTECTION {
        return true;
    }

    if vector == 32 {
        let window = crate::generated_consts::CORE_IRQ_STORM_WINDOW_TICKS.max(1);
        let tick = TIMER_TICKS.fetch_add(1, Ordering::Relaxed) + 1;
        if tick % window == 0 {
            IRQ_EPOCH.fetch_add(1, Ordering::Relaxed);
        }
    }

    let idx = vector as usize;
    if idx >= IRQ_SLOTS {
        return true;
    }

    let epoch = IRQ_EPOCH.load(Ordering::Relaxed);
    let prev = LAST_EPOCH[idx].load(Ordering::Relaxed);
    if prev != epoch {
        LAST_EPOCH[idx].store(epoch, Ordering::Relaxed);
        IRQ_COUNT[idx].store(0, Ordering::Relaxed);
    }

    let count = IRQ_COUNT[idx].fetch_add(1, Ordering::Relaxed) + 1;
    if count > crate::generated_consts::CORE_IRQ_STORM_THRESHOLD {
        #[cfg(feature = "rtos_strict")]
        panic!("RTOS Strict Violation: Interrupt execution threshold exceeded on vector {}. Potential deterministic breach.", vector);

        #[cfg(not(feature = "rtos_strict"))]
        {
            IRQ_DROPPED[idx].fetch_add(1, Ordering::Relaxed);
            if count == crate::generated_consts::CORE_IRQ_STORM_THRESHOLD + 1 {
                crate::klog_warn!(
                    "irq storm detected vector={} threshold={} window_ticks={}",
                    vector,
                    crate::generated_consts::CORE_IRQ_STORM_THRESHOLD,
                    crate::generated_consts::CORE_IRQ_STORM_WINDOW_TICKS
                );
            }
            return false;
        }
    }

    true
}

#[inline(always)]
pub fn dropped(vector: u8) -> u64 {
    IRQ_DROPPED[vector as usize].load(Ordering::Relaxed)
}

#[inline(always)]
pub fn dropped_for(vector: u8) -> u64 {
    dropped(vector)
}
