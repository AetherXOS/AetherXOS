use crate::config::{KernelConfig, VirtualizationExecutionClass, VirtualizationGovernorClass};
use crate::hal::common::virt::{
    GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_BALANCED, GOVERNOR_BIAS_RELAXED,
    GOVERNOR_CLASS_BACKGROUND_OPTIMIZED, GOVERNOR_CLASS_LATENCY_FOCUSED, GOVERNOR_ENERGY_BALANCED,
    GOVERNOR_ENERGY_PERFORMANCE, GOVERNOR_ENERGY_SAVING,
};

#[inline(always)]
pub fn virtualization_effective_execution_contract_holds() -> bool {
    let effective = KernelConfig::virtualization_effective_execution_profile();
    if KernelConfig::is_virtualization_enabled() {
        matches!(
            effective.scheduling_class,
            VirtualizationExecutionClass::LatencyCritical
                | VirtualizationExecutionClass::Balanced
                | VirtualizationExecutionClass::Background
        )
    } else {
        matches!(
            effective.scheduling_class,
            VirtualizationExecutionClass::Background
        )
    }
}

#[inline(always)]
pub fn virtualization_effective_governor_contract_holds() -> bool {
    let effective = KernelConfig::virtualization_effective_governor_profile();
    if KernelConfig::is_virtualization_enabled() {
        matches!(
            effective.governor_class,
            VirtualizationGovernorClass::Performance
                | VirtualizationGovernorClass::Balanced
                | VirtualizationGovernorClass::Efficiency
        )
    } else {
        matches!(
            effective.governor_class,
            VirtualizationGovernorClass::Efficiency
        )
    }
}

#[inline(always)]
pub fn virtualization_governor_bias_contract_holds(
    governor_class: &'static str,
    latency_bias: &'static str,
    energy_bias: &'static str,
) -> bool {
    match latency_bias {
        GOVERNOR_BIAS_AGGRESSIVE => {
            energy_bias == GOVERNOR_ENERGY_PERFORMANCE
                && governor_class != GOVERNOR_CLASS_BACKGROUND_OPTIMIZED
        }
        GOVERNOR_BIAS_RELAXED => {
            energy_bias == GOVERNOR_ENERGY_SAVING
                && governor_class != GOVERNOR_CLASS_LATENCY_FOCUSED
        }
        GOVERNOR_BIAS_BALANCED => energy_bias == GOVERNOR_ENERGY_BALANCED,
        _ => false,
    }
}
