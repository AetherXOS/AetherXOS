use crate::interfaces::task::ProcessId;
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::{BTreeMap, VecDeque};
use core::sync::atomic::{AtomicU64, Ordering};

const MAX_PENDING_DELIVERIES_PER_PROCESS: usize = 128;
const UPCALL_FLAG_VIRTUAL_IRQ: u32 = 1 << 31;

static UPCALL_REGISTER_CALLS: AtomicU64 = AtomicU64::new(0);
static UPCALL_REGISTER_OVERWRITES: AtomicU64 = AtomicU64::new(0);
static UPCALL_UNREGISTER_CALLS: AtomicU64 = AtomicU64::new(0);
static UPCALL_UNREGISTER_HITS: AtomicU64 = AtomicU64::new(0);
static UPCALL_RESOLVE_CALLS: AtomicU64 = AtomicU64::new(0);
static UPCALL_RESOLVE_HITS: AtomicU64 = AtomicU64::new(0);
static UPCALL_DELIVER_MARKS: AtomicU64 = AtomicU64::new(0);
static UPCALL_DELIVER_ENQUEUED: AtomicU64 = AtomicU64::new(0);
static UPCALL_DELIVER_QUEUE_DROPS: AtomicU64 = AtomicU64::new(0);
static UPCALL_CONSUME_CALLS: AtomicU64 = AtomicU64::new(0);
static UPCALL_CONSUME_HITS: AtomicU64 = AtomicU64::new(0);
static UPCALL_VIRQ_INJECT_CALLS: AtomicU64 = AtomicU64::new(0);
static UPCALL_VIRQ_INJECT_HITS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct UpcallStats {
    pub register_calls: u64,
    pub register_overwrites: u64,
    pub unregister_calls: u64,
    pub unregister_hits: u64,
    pub resolve_calls: u64,
    pub resolve_hits: u64,
    pub delivery_marks: u64,
    pub delivery_enqueued: u64,
    pub delivery_queue_drops: u64,
    pub consume_calls: u64,
    pub consume_hits: u64,
    pub virq_inject_calls: u64,
    pub virq_inject_hits: u64,
    pub pending_processes: usize,
    pub pending_deliveries: usize,
}

pub fn stats() -> UpcallStats {
    let (pending_processes, pending_deliveries) = GLOBAL_UPCALL_REGISTRY.pending_totals();
    UpcallStats {
        register_calls: UPCALL_REGISTER_CALLS.load(Ordering::Relaxed),
        register_overwrites: UPCALL_REGISTER_OVERWRITES.load(Ordering::Relaxed),
        unregister_calls: UPCALL_UNREGISTER_CALLS.load(Ordering::Relaxed),
        unregister_hits: UPCALL_UNREGISTER_HITS.load(Ordering::Relaxed),
        resolve_calls: UPCALL_RESOLVE_CALLS.load(Ordering::Relaxed),
        resolve_hits: UPCALL_RESOLVE_HITS.load(Ordering::Relaxed),
        delivery_marks: UPCALL_DELIVER_MARKS.load(Ordering::Relaxed),
        delivery_enqueued: UPCALL_DELIVER_ENQUEUED.load(Ordering::Relaxed),
        delivery_queue_drops: UPCALL_DELIVER_QUEUE_DROPS.load(Ordering::Relaxed),
        consume_calls: UPCALL_CONSUME_CALLS.load(Ordering::Relaxed),
        consume_hits: UPCALL_CONSUME_HITS.load(Ordering::Relaxed),
        virq_inject_calls: UPCALL_VIRQ_INJECT_CALLS.load(Ordering::Relaxed),
        virq_inject_hits: UPCALL_VIRQ_INJECT_HITS.load(Ordering::Relaxed),
        pending_processes,
        pending_deliveries,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpcallEntry {
    pub process_id: ProcessId,
    pub entry_pc: u64,
    pub user_ctx: u64,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpcallDelivery {
    pub irq: u8,
    pub process_id: ProcessId,
    pub entry_pc: u64,
    pub user_ctx: u64,
    pub flags: u32,
}

pub struct UpcallRegistry {
    table: IrqSafeMutex<BTreeMap<u8, UpcallEntry>>,
    pending: IrqSafeMutex<BTreeMap<ProcessId, VecDeque<UpcallDelivery>>>,
}

static GLOBAL_UPCALL_REGISTRY: UpcallRegistry = UpcallRegistry::new();

#[inline(always)]
pub fn global_registry() -> &'static UpcallRegistry {
    &GLOBAL_UPCALL_REGISTRY
}

#[inline(always)]
pub fn register_global(
    irq: u8,
    process_id: ProcessId,
    entry_pc: u64,
    user_ctx: u64,
    flags: u32,
) -> Option<UpcallEntry> {
    GLOBAL_UPCALL_REGISTRY.register(irq, process_id, entry_pc, user_ctx, flags)
}

#[inline(always)]
pub fn unregister_global_for_process(irq: u8, process_id: ProcessId) -> bool {
    GLOBAL_UPCALL_REGISTRY.unregister_for_process(irq, process_id)
}

#[inline(always)]
pub fn resolve_global(irq: u8) -> Option<UpcallDelivery> {
    GLOBAL_UPCALL_REGISTRY.resolve(irq)
}

#[inline(always)]
pub fn mark_global_delivered_for_irq(irq: u8) -> bool {
    GLOBAL_UPCALL_REGISTRY.mark_delivered_for_irq(irq)
}

#[inline(always)]
pub fn consume_global_for_process(process_id: ProcessId) -> Option<UpcallDelivery> {
    GLOBAL_UPCALL_REGISTRY.consume_for_process(process_id)
}

#[inline(always)]
pub fn inject_global_virtual_irq(
    process_id: ProcessId,
    irq: u8,
    user_ctx: u64,
    flags: u32,
) -> bool {
    GLOBAL_UPCALL_REGISTRY.inject_virtual_irq_for_process(process_id, irq, user_ctx, flags)
}

impl UpcallRegistry {
    pub const fn new() -> Self {
        Self {
            table: IrqSafeMutex::new(BTreeMap::new()),
            pending: IrqSafeMutex::new(BTreeMap::new()),
        }
    }

    pub fn register(
        &self,
        irq: u8,
        process_id: ProcessId,
        entry_pc: u64,
        user_ctx: u64,
        flags: u32,
    ) -> Option<UpcallEntry> {
        UPCALL_REGISTER_CALLS.fetch_add(1, Ordering::Relaxed);
        let mut map = self.table.lock();
        let replaced = map.insert(
            irq,
            UpcallEntry {
                process_id,
                entry_pc,
                user_ctx,
                flags,
            },
        );
        if replaced.is_some() {
            UPCALL_REGISTER_OVERWRITES.fetch_add(1, Ordering::Relaxed);
        }
        replaced
    }

    pub fn unregister_for_process(&self, irq: u8, process_id: ProcessId) -> bool {
        UPCALL_UNREGISTER_CALLS.fetch_add(1, Ordering::Relaxed);
        let mut map = self.table.lock();
        let allowed = map
            .get(&irq)
            .map(|entry| entry.process_id == process_id)
            .unwrap_or(false);
        if !allowed {
            return false;
        }
        let removed = map.remove(&irq).is_some();
        if removed {
            UPCALL_UNREGISTER_HITS.fetch_add(1, Ordering::Relaxed);
        }
        removed
    }

    pub fn resolve(&self, irq: u8) -> Option<UpcallDelivery> {
        UPCALL_RESOLVE_CALLS.fetch_add(1, Ordering::Relaxed);
        let maybe = self.table.lock().get(&irq).copied();
        if let Some(entry) = maybe {
            UPCALL_RESOLVE_HITS.fetch_add(1, Ordering::Relaxed);
            Some(UpcallDelivery {
                irq,
                process_id: entry.process_id,
                entry_pc: entry.entry_pc,
                user_ctx: entry.user_ctx,
                flags: entry.flags,
            })
        } else {
            None
        }
    }

    fn queue_delivery(&self, delivery: UpcallDelivery) -> bool {
        let mut pending = self.pending.lock();
        let queue = pending
            .entry(delivery.process_id)
            .or_insert_with(VecDeque::new);
        if queue.len() >= MAX_PENDING_DELIVERIES_PER_PROCESS {
            UPCALL_DELIVER_QUEUE_DROPS.fetch_add(1, Ordering::Relaxed);
            return false;
        }
        queue.push_back(delivery);
        UPCALL_DELIVER_ENQUEUED.fetch_add(1, Ordering::Relaxed);
        true
    }

    pub fn mark_delivered_for_irq(&self, irq: u8) -> bool {
        UPCALL_DELIVER_MARKS.fetch_add(1, Ordering::Relaxed);
        let Some(delivery) = self.resolve(irq) else {
            return false;
        };
        self.queue_delivery(delivery)
    }

    pub fn consume_for_process(&self, process_id: ProcessId) -> Option<UpcallDelivery> {
        UPCALL_CONSUME_CALLS.fetch_add(1, Ordering::Relaxed);
        let mut pending = self.pending.lock();
        let queue = pending.get_mut(&process_id)?;
        let item = queue.pop_front();
        if item.is_some() {
            UPCALL_CONSUME_HITS.fetch_add(1, Ordering::Relaxed);
        }
        if queue.is_empty() {
            pending.remove(&process_id);
        }
        item
    }

    pub fn inject_virtual_irq_for_process(
        &self,
        process_id: ProcessId,
        irq: u8,
        user_ctx: u64,
        flags: u32,
    ) -> bool {
        UPCALL_VIRQ_INJECT_CALLS.fetch_add(1, Ordering::Relaxed);
        let Some(entry) = self.resolve(irq) else {
            return false;
        };
        if entry.process_id != process_id {
            return false;
        }

        let delivery = UpcallDelivery {
            irq,
            process_id,
            entry_pc: entry.entry_pc,
            user_ctx,
            flags: entry.flags | flags | UPCALL_FLAG_VIRTUAL_IRQ,
        };

        if self.queue_delivery(delivery) {
            UPCALL_VIRQ_INJECT_HITS.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    pub fn pending_totals(&self) -> (usize, usize) {
        let pending = self.pending.lock();
        let processes = pending.len();
        let deliveries = pending.values().map(|q| q.len()).sum();
        (processes, deliveries)
    }

    pub fn len(&self) -> usize {
        self.table.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn upcall_register_and_resolve_roundtrip() {
        let reg = UpcallRegistry::new();
        let pid = ProcessId(1001);
        assert!(reg.register(33, pid, 0x4000, 0x77, 0).is_none());
        let d = reg.resolve(33);
        assert!(d.is_some());
        let d = d.unwrap_or(UpcallDelivery {
            irq: 0,
            process_id: pid,
            entry_pc: 0,
            user_ctx: 0,
            flags: 0,
        });
        assert_eq!(d.irq, 33);
        assert_eq!(d.process_id, pid);
        assert_eq!(d.entry_pc, 0x4000);
        assert_eq!(d.user_ctx, 0x77);
    }

    #[test_case]
    fn upcall_unregister_enforces_ownership() {
        let reg = UpcallRegistry::new();
        let pid7 = ProcessId(7);
        let pid9 = ProcessId(9);
        let _ = reg.register(55, pid7, 0x1000, 0, 0);
        assert!(!reg.unregister_for_process(55, pid9));
        assert!(reg.unregister_for_process(55, pid7));
        assert!(reg.resolve(55).is_none());
    }

    #[test_case]
    fn upcall_register_overwrite_replaces_previous_owner() {
        let reg = UpcallRegistry::new();
        let pid1 = ProcessId(1);
        let pid2 = ProcessId(2);
        let _ = reg.register(70, pid1, 0x1111, 1, 0);
        let replaced = reg.register(70, pid2, 0x2222, 2, 0);
        assert!(replaced.is_some());
        let d = reg.resolve(70);
        assert!(d.is_some());
        assert_eq!(
            d.unwrap_or(UpcallDelivery {
                irq: 0,
                process_id: pid2,
                entry_pc: 0,
                user_ctx: 0,
                flags: 0,
            })
            .process_id,
            pid2
        );
    }

    #[test_case]
    fn upcall_delivery_enqueue_and_consume_roundtrip() {
        let reg = UpcallRegistry::new();
        let pid42 = ProcessId(42);
        let _ = reg.register(10, pid42, 0x8000, 0x22, 0);
        assert!(reg.mark_delivered_for_irq(10));

        let msg = reg.consume_for_process(pid42);
        assert!(msg.is_some());
        let msg = msg.unwrap_or(UpcallDelivery {
            irq: 0,
            process_id: pid42,
            entry_pc: 0,
            user_ctx: 0,
            flags: 0,
        });
        assert_eq!(msg.irq, 10);
        assert_eq!(msg.process_id, pid42);
        assert_eq!(msg.entry_pc, 0x8000);
        assert_eq!(msg.user_ctx, 0x22);
    }

    #[test_case]
    fn upcall_virtual_irq_injection_sets_virtual_flag() {
        let reg = UpcallRegistry::new();
        let pid99 = ProcessId(99);
        let _ = reg.register(9, pid99, 0x1110, 0x0, 0x10);
        assert!(reg.inject_virtual_irq_for_process(pid99, 9, 0x55, 0x20));

        let msg = reg.consume_for_process(pid99);
        assert!(msg.is_some());
        let msg = msg.unwrap_or(UpcallDelivery {
            irq: 0,
            process_id: pid99,
            entry_pc: 0,
            user_ctx: 0,
            flags: 0,
        });
        assert_eq!(msg.user_ctx, 0x55);
        assert_eq!(msg.flags & UPCALL_FLAG_VIRTUAL_IRQ, UPCALL_FLAG_VIRTUAL_IRQ);
    }
}
