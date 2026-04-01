#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitFallbackKind {
    Drop,
    WarnAndContinue,
}

#[derive(Debug, Clone, Copy)]
pub struct WaitPolicyDescriptor {
    pub component: &'static str,
    pub operation: &'static str,
    pub max_spins: usize,
    pub fallback: WaitFallbackKind,
    pub timeout_events: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct HalWaitPolicySnapshot {
    pub serial_tx: WaitPolicyDescriptor,
    pub smp_boot: WaitPolicyDescriptor,
    pub tlb_shootdown: WaitPolicyDescriptor,
    pub iommu_mmio: WaitPolicyDescriptor,
}

#[cfg(target_arch = "x86_64")]
fn arch_serial_descriptor() -> WaitPolicyDescriptor {
    let s = crate::hal::x86_64::serial::stats();
    WaitPolicyDescriptor {
        component: "serial",
        operation: "tx_fifo_wait",
        max_spins: crate::hal::x86_64::serial::tx_timeout_spins(),
        fallback: WaitFallbackKind::Drop,
        timeout_events: s.tx_timeouts,
    }
}

#[cfg(target_arch = "aarch64")]
fn arch_serial_descriptor() -> WaitPolicyDescriptor {
    let s = crate::hal::aarch64::serial::stats();
    WaitPolicyDescriptor {
        component: "serial",
        operation: "tx_fifo_wait",
        max_spins: crate::hal::aarch64::serial::tx_timeout_spins(),
        fallback: WaitFallbackKind::Drop,
        timeout_events: s.tx_timeouts,
    }
}

#[cfg(target_arch = "x86_64")]
fn arch_wait_descriptor_pair() -> (WaitPolicyDescriptor, WaitPolicyDescriptor) {
    let s = crate::hal::x86_64::smp::wait_stats();
    (
        WaitPolicyDescriptor {
            component: "smp",
            operation: "ap_boot_wait",
            max_spins: s.boot_timeout_spins,
            fallback: WaitFallbackKind::WarnAndContinue,
            timeout_events: s.boot_timeouts,
        },
        WaitPolicyDescriptor {
            component: "smp",
            operation: "tlb_shootdown_wait",
            max_spins: s.tlb_shootdown_timeout_spins,
            fallback: WaitFallbackKind::WarnAndContinue,
            timeout_events: s.tlb_shootdown_timeouts,
        },
    )
}

#[cfg(target_arch = "aarch64")]
fn arch_wait_descriptor_pair() -> (WaitPolicyDescriptor, WaitPolicyDescriptor) {
    let s = crate::hal::aarch64::smp::wait_stats();
    (
        WaitPolicyDescriptor {
            component: "smp",
            operation: "ap_boot_wait",
            max_spins: s.boot_timeout_spins,
            fallback: WaitFallbackKind::WarnAndContinue,
            timeout_events: s.boot_timeouts,
        },
        WaitPolicyDescriptor {
            component: "smp",
            operation: "tlb_shootdown_wait",
            max_spins: s.tlb_shootdown_timeout_spins,
            fallback: WaitFallbackKind::WarnAndContinue,
            timeout_events: s.tlb_shootdown_timeouts,
        },
    )
}

pub fn snapshot() -> HalWaitPolicySnapshot {
    let iommu = crate::hal::iommu::stats();
    let serial_tx = arch_serial_descriptor();
    let (smp_boot, tlb_shootdown) = arch_wait_descriptor_pair();
    HalWaitPolicySnapshot {
        serial_tx,
        smp_boot,
        tlb_shootdown,
        iommu_mmio: WaitPolicyDescriptor {
            component: "iommu",
            operation: "mmio_invalidation_wait",
            max_spins: crate::hal::iommu::wait_timeout_spins(),
            fallback: WaitFallbackKind::WarnAndContinue,
            timeout_events: iommu.amdvi_inv_timeout_count,
        },
    }
}
