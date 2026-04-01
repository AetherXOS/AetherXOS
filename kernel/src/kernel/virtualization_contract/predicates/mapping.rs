use crate::config::{VirtualizationExecutionClass, VirtualizationGovernorClass};
use crate::hal::common::virt::GOVERNOR_CLASS_EFFICIENCY;

#[inline(always)]
pub fn expected_runtime_governor_class(
    governor_class: VirtualizationGovernorClass,
) -> &'static str {
    match governor_class {
        VirtualizationGovernorClass::Performance => {
            crate::hal::common::virt::GOVERNOR_CLASS_PERFORMANCE
        }
        VirtualizationGovernorClass::Balanced => crate::hal::common::virt::GOVERNOR_CLASS_BALANCED,
        VirtualizationGovernorClass::Efficiency => GOVERNOR_CLASS_EFFICIENCY,
    }
}

#[inline(always)]
pub fn execution_profile_matches_status(
    execution_class: VirtualizationExecutionClass,
    execution_profile: &'static str,
) -> bool {
    execution_class.as_str() == execution_profile
}
