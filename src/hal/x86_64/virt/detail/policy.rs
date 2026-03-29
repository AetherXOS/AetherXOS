use crate::hal::common::virt::{
    VirtStatus, CAP_TIER_0, CAP_TIER_1, CAP_TIER_2, CAP_TIER_3, ISOLATION_TIER_DMA_ISOLATED,
    ISOLATION_TIER_IOMMU_READY, ISOLATION_TIER_NONE, ISOLATION_TIER_SHADOW_ONLY,
};

pub(super) fn capability_level(
    status: VirtStatus,
    trap_handling_ready: bool,
    memory_isolation_ready: bool,
) -> &'static str {
    let policy = crate::config::KernelConfig::virtualization_effective_profile();
    if status.vm_launch_ready && memory_isolation_ready && policy.snapshot && policy.live_migration
    {
        CAP_TIER_3
    } else if trap_handling_ready {
        CAP_TIER_2
    } else if status.vm_launch_ready
        || status.enabled.vmx_enabled
        || status.enabled.svm_enabled
        || status.caps.vmx
        || status.caps.svm
    {
        CAP_TIER_1
    } else {
        CAP_TIER_0
    }
}

pub(super) fn isolation_tier(
    status: VirtStatus,
    memory_isolation_ready: bool,
    attached_devices: usize,
) -> &'static str {
    if memory_isolation_ready && attached_devices > 0 {
        ISOLATION_TIER_DMA_ISOLATED
    } else if memory_isolation_ready {
        ISOLATION_TIER_IOMMU_READY
    } else if status.vm_launch_ready {
        ISOLATION_TIER_SHADOW_ONLY
    } else {
        ISOLATION_TIER_NONE
    }
}
