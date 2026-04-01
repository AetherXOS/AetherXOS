pub const READINESS_READY: &str = "ready";
pub const READINESS_STAGED: &str = "staged";
pub const READINESS_PARTIAL: &str = "partial";
pub const READINESS_BLOCKED: &str = "blocked";
pub const READINESS_POLICY_LIMITED: &str = "policy-limited";

pub const STAGE_GUEST_RUNNABLE: &str = "guest-runnable";
pub const STAGE_LAUNCH_PREPARED: &str = "launch-prepared";
pub const STAGE_CONTROL_PLANE_READY: &str = "control-plane-ready";
pub const STAGE_HARDWARE_ENABLED: &str = "hardware-enabled";
pub const STAGE_UNAVAILABLE: &str = "unavailable";

pub const CAP_TIER_0: &str = "tier0";
pub const CAP_TIER_1: &str = "tier1";
pub const CAP_TIER_2: &str = "tier2";
pub const CAP_TIER_3: &str = "tier3";

pub const OBS_TIER_MINIMAL: &str = "minimal";
pub const OBS_TIER_PARTIAL: &str = "partial";
pub const OBS_TIER_FULL: &str = "full";

pub const ADVANCED_TIER_BASELINE: &str = "baseline";
pub const ADVANCED_TIER_ADVANCED: &str = "advanced";
pub const ADVANCED_TIER_HYPERVISOR_GRADE: &str = "hypervisor-grade";

pub const ISOLATION_TIER_DMA_ISOLATED: &str = "dma-isolated";
pub const ISOLATION_TIER_IOMMU_READY: &str = "iommu-ready";
pub const ISOLATION_TIER_STAGE2_GICV3: &str = "stage2-gicv3";
pub const ISOLATION_TIER_STAGE2_READY: &str = "stage2-ready";
pub const ISOLATION_TIER_SHADOW_ONLY: &str = "shadow-only";
pub const ISOLATION_TIER_STAGE1_ONLY: &str = "stage1-only";
pub const ISOLATION_TIER_NONE: &str = "none";

pub const OPERATIONAL_TIER_PRODUCTION: &str = "production";
pub const OPERATIONAL_TIER_DEGRADED: &str = "degraded";
pub const OPERATIONAL_TIER_RESTRICTED: &str = "restricted";
pub const OPERATIONAL_TIER_UNAVAILABLE: &str = "unavailable";

#[inline(always)]
pub fn detail_operation_class(readiness: &'static str, policy_scope: &'static str) -> &'static str {
    if matches!(readiness, READINESS_BLOCKED | READINESS_POLICY_LIMITED) {
        super::runtime::OPERATION_CLASS_BLOCKED
    } else if readiness != READINESS_READY || policy_scope != "fully-enabled" {
        super::runtime::OPERATION_CLASS_BASIC
    } else {
        super::runtime::OPERATION_CLASS_FULL
    }
}

#[inline(always)]
pub fn operational_tier_from_class(
    readiness: &'static str,
    capability_level: &'static str,
    operation_class: &'static str,
    policy_scope: &'static str,
) -> &'static str {
    if readiness == READINESS_BLOCKED || capability_level == CAP_TIER_0 {
        OPERATIONAL_TIER_UNAVAILABLE
    } else if operation_class == super::runtime::OPERATION_CLASS_BLOCKED {
        OPERATIONAL_TIER_RESTRICTED
    } else if operation_class == super::runtime::OPERATION_CLASS_BASIC
        || policy_scope != "fully-enabled"
        || capability_level != CAP_TIER_3
    {
        OPERATIONAL_TIER_DEGRADED
    } else {
        OPERATIONAL_TIER_PRODUCTION
    }
}
