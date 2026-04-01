use crate::hal::common::virt::{
    VirtStatus, CAP_TIER_0, CAP_TIER_1, CAP_TIER_2, CAP_TIER_3, ISOLATION_TIER_NONE,
    ISOLATION_TIER_STAGE1_ONLY, ISOLATION_TIER_STAGE2_GICV3, ISOLATION_TIER_STAGE2_READY,
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
    } else if status.vm_launch_ready || status.caps.hypervisor_present {
        CAP_TIER_1
    } else {
        CAP_TIER_0
    }
}

pub(super) fn isolation_tier(
    status: VirtStatus,
    memory_isolation_ready: bool,
    gic_version: u32,
) -> &'static str {
    if memory_isolation_ready && gic_version >= 3 {
        ISOLATION_TIER_STAGE2_GICV3
    } else if memory_isolation_ready {
        ISOLATION_TIER_STAGE2_READY
    } else if status.vm_launch_ready {
        ISOLATION_TIER_STAGE1_ONLY
    } else {
        ISOLATION_TIER_NONE
    }
}
