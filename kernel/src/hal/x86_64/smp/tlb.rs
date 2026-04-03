use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::hal::common::ipi::{acknowledge_pending, wait_for_pending_acks};
use crate::hal::x86_64::apic;

static SHOOTDOWN_ADDR: AtomicU64 = AtomicU64::new(0);
static SHOOTDOWN_PENDING: AtomicUsize = AtomicUsize::new(0);

pub(super) const IPI_TLB_SHOOTDOWN_VECTOR: u8 = 253;

pub(super) fn broadcast_tlb_shootdown(addr: u64, cpu_count: usize, timeout_spins: usize, timeout_counter: &AtomicU64) {
    use x86_64::VirtAddr;

    x86_64::instructions::tlb::flush(VirtAddr::new(addr));

    if cpu_count <= 1 {
        return;
    }

    SHOOTDOWN_ADDR.store(addr, Ordering::Release);
    SHOOTDOWN_PENDING.store(cpu_count - 1, Ordering::Release);

    unsafe {
        apic::send_ipi_all_excluding_self(IPI_TLB_SHOOTDOWN_VECTOR);
    }

    if let Some(left) = wait_for_pending_acks(&SHOOTDOWN_PENDING, timeout_spins) {
        timeout_counter.fetch_add(1, Ordering::Relaxed);
        crate::klog_warn!(
            "x86_64 TLB shootdown timeout: {} AP(s) did not respond",
            left
        );
        SHOOTDOWN_PENDING.store(0, Ordering::Release);
    }
}

pub(super) fn handle_tlb_shootdown() {
    use x86_64::VirtAddr;

    let addr = SHOOTDOWN_ADDR.load(Ordering::Acquire);
    unsafe {
        x86_64::instructions::tlb::flush(VirtAddr::new(addr));
        apic::eoi();
    }
    acknowledge_pending(&SHOOTDOWN_PENDING);
}
