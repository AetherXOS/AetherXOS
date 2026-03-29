use super::*;

mod lifecycle_ops;
mod profiles;
mod status;

pub(super) use lifecycle_ops::{
    initialize_launch_context, reset_launch_context, teardown_launch_context,
};
pub(super) use profiles::{
    advanced_operations_profile, dirty_logging_ready, guest_control_profile, guest_exit_profile,
    guest_launch_profile, guest_lifecycle_profile, guest_operation_profile, guest_runtime_profile,
    live_migration_ready, snapshot_ready,
};
pub(super) use status::status;

pub(super) fn blocker_reason(code: u8) -> &'static str {
    crate::hal::common::virt::el2_blocker_label(code)
}

pub(super) fn lifecycle_reason(code: u8) -> &'static str {
    crate::hal::common::virt::lifecycle_label(code)
}

#[cfg(test)]
mod tests {
    use super::{blocker_reason, lifecycle_reason};

    #[test_case]
    fn blocker_strings_are_stable() {
        assert_eq!(blocker_reason(0), "None");
        assert_eq!(blocker_reason(1), "EL2 Not Supported");
        assert_eq!(blocker_reason(2), "EL2 Not Active");
    }

    #[test_case]
    fn lifecycle_strings_are_stable() {
        assert_eq!(lifecycle_reason(0), "uninitialized");
        assert_eq!(lifecycle_reason(1), "prepared");
        assert_eq!(lifecycle_reason(2), "active");
        assert_eq!(lifecycle_reason(3), "torn-down");
        assert_eq!(lifecycle_reason(4), "failed");
    }
}

