#[path = "control_plane_support/parse.rs"]
mod parse;
#[cfg(feature = "vfs")]
#[path = "control_plane_support/vfs_constants.rs"]
mod vfs_constants;
#[cfg(feature = "vfs")]
#[path = "control_plane_support/vfs_matrix_data.rs"]
mod vfs_matrix_data;
#[cfg(feature = "vfs")]
#[path = "control_plane_support/reports.rs"]
mod reports;
#[cfg(feature = "vfs")]
#[path = "control_plane_support/vfs_reports.rs"]
mod vfs_reports;
#[cfg(all(test, target_os = "none"))]
#[path = "control_plane_support/tests.rs"]
mod tests;

pub(super) use parse::{parse_override_entry, split_override_entries};
#[cfg(feature = "vfs")]
pub(crate) use reports::{
    build_feature_report, build_feature_summary_report, build_linux_compat_readiness_report,
    build_runtime_keys_report, build_security_posture_report, build_snapshot_report,
    normalize_export_dir,
};
#[cfg(feature = "vfs")]
pub(crate) use vfs_reports::{
    build_vfs_behavior_report, build_vfs_focus_report, build_vfs_matrix_report,
};
