use super::support;

pub fn snapshot_ready() -> bool {
    support::snapshot_ready()
}

pub fn dirty_logging_ready() -> bool {
    support::dirty_logging_ready()
}

pub fn live_migration_ready() -> bool {
    support::live_migration_ready()
}

pub fn advanced_operations_profile() -> (bool, bool, bool, &'static str) {
    support::advanced_operations_profile()
}

pub fn guest_lifecycle_profile() -> (&'static str, bool, bool, &'static str) {
    support::guest_lifecycle_profile()
}

pub fn guest_control_profile() -> (&'static str, bool, bool, bool) {
    support::guest_control_profile()
}

pub fn guest_runtime_profile() -> (&'static str, bool, bool, bool, bool) {
    support::guest_runtime_profile()
}

pub fn guest_exit_profile() -> (&'static str, bool, bool, bool, bool) {
    support::guest_exit_profile()
}

pub fn guest_launch_profile() -> (&'static str, bool, bool, bool) {
    support::guest_launch_profile()
}

pub fn guest_operation_profile() -> crate::hal::common::virt::GuestOperationProfile {
    support::guest_operation_profile()
}
