use super::VirtPlatformStatus;

#[derive(Debug, Clone, Copy)]
pub(super) struct VirtLifecycleSnapshot {
    pub summary: &'static str,
    pub progress_per_mille: u16,
}

impl VirtPlatformStatus {
    pub(super) fn lifecycle_snapshot(self) -> VirtLifecycleSnapshot {
        let policy = crate::config::KernelConfig::virtualization_effective_profile();

        fn state_is_progress(state: &'static str) -> bool {
            matches!(state, "detected" | "prepared" | "active" | "ready")
        }

        let completed_steps = [
            self.detect_state,
            self.prepare_state,
            self.capability_state,
            self.feature_state,
            self.launch_state,
            self.resume_state,
            self.trap_state,
        ]
        .into_iter()
        .filter(|state| state_is_progress(*state))
        .count() as u16;

        let summary = crate::hal::common::virt::lifecycle_summary_from_states(
            policy,
            self.detect_state,
            self.prepare_state,
            self.capability_state,
            self.feature_state,
            self.launch_state,
            self.resume_state,
            self.trap_state,
        );

        VirtLifecycleSnapshot {
            summary,
            progress_per_mille: crate::hal::common::virt::lifecycle_progress_per_mille(
                completed_steps,
                policy,
            ),
        }
    }
}
