use core::sync::atomic::Ordering;

use super::*;

impl KernelConfig {
    #[inline(always)]
    fn virtualization_execution_class_from_override(
        value: usize,
    ) -> Option<VirtualizationExecutionClass> {
        match value {
            1 => Some(VirtualizationExecutionClass::LatencyCritical),
            2 => Some(VirtualizationExecutionClass::Balanced),
            3 => Some(VirtualizationExecutionClass::Background),
            _ => None,
        }
    }

    #[inline(always)]
    fn virtualization_execution_class_override_value(
        value: Option<VirtualizationExecutionClass>,
    ) -> usize {
        match value {
            None => 0,
            Some(VirtualizationExecutionClass::LatencyCritical) => 1,
            Some(VirtualizationExecutionClass::Balanced) => 2,
            Some(VirtualizationExecutionClass::Background) => 3,
        }
    }

    #[inline(always)]
    fn virtualization_governor_class_from_override(
        value: usize,
    ) -> Option<VirtualizationGovernorClass> {
        match value {
            1 => Some(VirtualizationGovernorClass::Performance),
            2 => Some(VirtualizationGovernorClass::Balanced),
            3 => Some(VirtualizationGovernorClass::Efficiency),
            _ => None,
        }
    }

    #[inline(always)]
    fn virtualization_governor_class_override_value(
        value: Option<VirtualizationGovernorClass>,
    ) -> usize {
        match value {
            None => 0,
            Some(VirtualizationGovernorClass::Performance) => 1,
            Some(VirtualizationGovernorClass::Balanced) => 2,
            Some(VirtualizationGovernorClass::Efficiency) => 3,
        }
    }

    pub fn virtualization_execution_profile() -> VirtualizationExecutionProfile {
        VirtualizationExecutionProfile {
            scheduling_class: Self::virtualization_execution_class_from_override(
                VIRTUALIZATION_EXECUTION_CLASS_OVERRIDE.load(Ordering::Relaxed),
            )
            .unwrap_or(VirtualizationExecutionClass::Balanced),
        }
    }

    pub fn virtualization_cargo_execution_profile() -> VirtualizationExecutionProfile {
        VirtualizationExecutionProfile {
            scheduling_class: VirtualizationExecutionClass::Balanced,
        }
    }

    pub fn virtualization_effective_execution_profile() -> VirtualizationExecutionProfile {
        let runtime = Self::virtualization_execution_profile();
        let cargo = Self::virtualization_cargo_execution_profile();
        VirtualizationExecutionProfile {
            scheduling_class: if Self::is_virtualization_enabled() {
                match cargo.scheduling_class {
                    VirtualizationExecutionClass::Balanced => runtime.scheduling_class,
                    _ => cargo.scheduling_class,
                }
            } else {
                VirtualizationExecutionClass::Background
            },
        }
    }

    pub fn virtualization_execution_policy_profile() -> VirtualizationExecutionPolicyProfile {
        let runtime = Self::virtualization_execution_profile();
        let cargo = Self::virtualization_cargo_execution_profile();
        VirtualizationExecutionPolicyProfile {
            runtime,
            cargo,
            effective: Self::virtualization_effective_execution_profile(),
        }
    }

    pub fn virtualization_governor_profile() -> VirtualizationGovernorProfile {
        VirtualizationGovernorProfile {
            governor_class: Self::virtualization_governor_class_from_override(
                VIRTUALIZATION_GOVERNOR_CLASS_OVERRIDE.load(Ordering::Relaxed),
            )
            .unwrap_or(VirtualizationGovernorClass::Balanced),
        }
    }

    pub fn virtualization_cargo_governor_profile() -> VirtualizationGovernorProfile {
        VirtualizationGovernorProfile {
            governor_class: VirtualizationGovernorClass::Balanced,
        }
    }

    pub fn virtualization_effective_governor_profile() -> VirtualizationGovernorProfile {
        let runtime = Self::virtualization_governor_profile();
        let cargo = Self::virtualization_cargo_governor_profile();
        VirtualizationGovernorProfile {
            governor_class: if Self::is_virtualization_enabled() {
                match cargo.governor_class {
                    VirtualizationGovernorClass::Balanced => runtime.governor_class,
                    _ => cargo.governor_class,
                }
            } else {
                VirtualizationGovernorClass::Efficiency
            },
        }
    }

    pub fn virtualization_governor_policy_profile() -> VirtualizationGovernorPolicyProfile {
        let runtime = Self::virtualization_governor_profile();
        let cargo = Self::virtualization_cargo_governor_profile();
        VirtualizationGovernorPolicyProfile {
            runtime,
            cargo,
            effective: Self::virtualization_effective_governor_profile(),
        }
    }

    pub fn set_virtualization_execution_profile(value: Option<VirtualizationExecutionProfile>) {
        VIRTUALIZATION_EXECUTION_CLASS_OVERRIDE.store(
            Self::virtualization_execution_class_override_value(
                value.map(|profile| profile.scheduling_class),
            ),
            Ordering::Relaxed,
        );
    }

    pub fn set_virtualization_governor_profile(value: Option<VirtualizationGovernorProfile>) {
        VIRTUALIZATION_GOVERNOR_CLASS_OVERRIDE.store(
            Self::virtualization_governor_class_override_value(
                value.map(|profile| profile.governor_class),
            ),
            Ordering::Relaxed,
        );
    }

    pub fn set_virtualization_execution_policy_profile(
        value: Option<VirtualizationExecutionProfile>,
    ) {
        apply_profile_override(
            value,
            |profile| {
                Self::set_virtualization_execution_profile(Some(profile));
            },
            || {
                Self::set_virtualization_execution_profile(None);
            },
        );
    }

    pub fn set_virtualization_governor_policy_profile(
        value: Option<VirtualizationGovernorProfile>,
    ) {
        apply_profile_override(
            value,
            |profile| {
                Self::set_virtualization_governor_profile(Some(profile));
            },
            || {
                Self::set_virtualization_governor_profile(None);
            },
        );
    }
}
