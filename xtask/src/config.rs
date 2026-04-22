//! Shared configuration and constants for the xtask runner.
//! Centralizing these avoids magic values and duplication across modules.

use std::path::PathBuf;

use crate::utils::paths;

#[allow(dead_code)]
pub fn kernel_compat_path() -> PathBuf {
    paths::kernel_src("modules/linux_compat")
}
#[allow(dead_code)]
pub fn kernel_shim_path() -> PathBuf {
    paths::kernel_src("kernel/syscalls/linux_shim")
}
#[allow(dead_code)]
pub fn syscall_consts_path() -> PathBuf {
    paths::kernel_src("kernel/syscalls/syscalls_consts.rs")
}
#[allow(dead_code)]
pub fn generated_consts_path() -> PathBuf {
    paths::kernel_src("generated_consts.rs")
}

pub mod repo_paths {
    #[allow(dead_code)]
    pub const ABI_GAP_SUMMARY: &str = "reports/abi_gap_inventory/summary.json";
    #[allow(dead_code)]
    pub const ERRNO_CONFORMANCE_SUMMARY: &str = "reports/errno_conformance/summary.json";
    #[allow(dead_code)]
    pub const SHIM_ERRNO_SUMMARY: &str = "reports/linux_shim_errno_conformance/summary.json";
    #[allow(dead_code)]
    pub const SYSCALL_COVERAGE_SUMMARY: &str = "reports/syscall_coverage_summary.json";
    #[allow(dead_code)]
    pub const ABI_READINESS_SUMMARY: &str = "reports/abi_readiness/summary.json";
    #[allow(dead_code)]
    pub const POSIX_CONFORMANCE_SUMMARY: &str = "reports/posix_conformance/summary.json";
    #[allow(dead_code)]
    pub const P_TIER_STATUS_JSON: &str = "reports/tooling/p_tier_status.json";
    #[allow(dead_code)]
    pub const P_TIER_STATUS_MD: &str = "reports/tooling/p_tier_status.md";
    #[allow(dead_code)]
    pub const PRODUCTION_ACCEPTANCE_SCORECARD_JSON: &str =
        "reports/tooling/production_release_acceptance_scorecard.json";
    #[allow(dead_code)]
    pub const PRODUCTION_ACCEPTANCE_SCORECARD_MD: &str =
        "reports/tooling/production_release_acceptance_scorecard.md";
    #[allow(dead_code)]
    pub const REPRO_BUILD_EVIDENCE_JSON: &str = "reports/tooling/reproducible_build_evidence.json";
    #[allow(dead_code)]
    pub const REPRO_BUILD_EVIDENCE_MD: &str = "reports/tooling/reproducible_build_evidence.md";
    #[allow(dead_code)]
    pub const REPRO_COMPARE_JSON: &str = "reports/tooling/reproducibility_compare.json";
    #[allow(dead_code)]
    pub const REPRO_COMPARE_MD: &str = "reports/tooling/reproducibility_compare.md";
    #[allow(dead_code)]
    pub const DOCS_COMMAND_AUDIT_JSON: &str = "reports/tooling/docs_command_audit.json";
    #[allow(dead_code)]
    pub const DOCS_COMMAND_AUDIT_MD: &str = "reports/tooling/docs_command_audit.md";
    #[allow(dead_code)]
    pub const RELEASE_EVIDENCE_BUNDLE_JSON: &str = "reports/tooling/release_evidence_bundle.json";
    #[allow(dead_code)]
    pub const RELEASE_EVIDENCE_BUNDLE_MD: &str = "reports/tooling/release_evidence_bundle.md";
    #[allow(dead_code)]
    pub const ABI_DRIFT_BASELINE_JSON: &str = "reports/tooling/abi_drift_baseline.json";
    #[allow(dead_code)]
    pub const ABI_DRIFT_REPORT_JSON: &str = "reports/tooling/abi_drift_report.json";
    #[allow(dead_code)]
    pub const ABI_DRIFT_REPORT_MD: &str = "reports/tooling/abi_drift_report.md";
    #[allow(dead_code)]
    pub const RELEASE_DIAGNOSTICS_JSON: &str = "reports/tooling/release_diagnostics.json";
    #[allow(dead_code)]
    pub const RELEASE_DIAGNOSTICS_MD: &str = "reports/tooling/release_diagnostics.md";
    #[allow(dead_code)]
    pub const HOST_TOOL_VERIFY_JSON: &str = "reports/tooling/host_tool_verify.json";
    #[allow(dead_code)]
    pub const HOST_TOOL_VERIFY_MD: &str = "reports/tooling/host_tool_verify.md";
    #[allow(dead_code)]
    pub const CRITICAL_POLICY_GUARD_JSON: &str = "reports/tooling/critical_policy_guard.json";
    #[allow(dead_code)]
    pub const CRITICAL_POLICY_GUARD_MD: &str = "reports/tooling/critical_policy_guard.md";
    #[allow(dead_code)]
    pub const WARNING_AUDIT_JSON: &str = "reports/tooling/warning_audit.json";
    #[allow(dead_code)]
    pub const WARNING_AUDIT_MD: &str = "reports/tooling/warning_audit.md";
    #[allow(dead_code)]
    pub const CI_BUNDLE_JSON: &str = "reports/tooling/ci_bundle.json";
    #[allow(dead_code)]
    pub const CI_BUNDLE_MD: &str = "reports/tooling/ci_bundle.md";
    #[allow(dead_code)]
    pub const RELEASE_DOCTOR_JSON: &str = "reports/tooling/release_doctor.json";
    #[allow(dead_code)]
    pub const RELEASE_DOCTOR_MD: &str = "reports/tooling/release_doctor.md";
    #[allow(dead_code)]
    pub const CI_GATE_REPORT_JSON: &str = "reports/tooling/ci_gate_report.json";
    #[allow(dead_code)]
    pub const CI_GATE_REPORT_MD: &str = "reports/tooling/ci_gate_report.md";
    #[allow(dead_code)]
    pub const RELEASE_GATES_JUNIT_XML: &str = "artifacts/release_gates_junit.xml";
    #[allow(dead_code)]
    pub const EXPLAIN_FAILURE_JSON: &str = "reports/tooling/explain_failure.json";
    #[allow(dead_code)]
    pub const EXPLAIN_FAILURE_MD: &str = "reports/tooling/explain_failure.md";
    #[allow(dead_code)]
    pub const TREND_HISTORY_JSON: &str = "reports/tooling/trend_history.json";
    #[allow(dead_code)]
    pub const TREND_DASHBOARD_JSON: &str = "reports/tooling/trend_dashboard.json";
    #[allow(dead_code)]
    pub const TREND_DASHBOARD_MD: &str = "reports/tooling/trend_dashboard.md";
    #[allow(dead_code)]
    pub const FREEZE_CHECK_JSON: &str = "reports/tooling/freeze_check.json";
    #[allow(dead_code)]
    pub const FREEZE_CHECK_MD: &str = "reports/tooling/freeze_check.md";
    #[allow(dead_code)]
    pub const SBOM_AUDIT_JSON: &str = "reports/tooling/sbom_audit.json";
    #[allow(dead_code)]
    pub const SBOM_AUDIT_MD: &str = "reports/tooling/sbom_audit.md";
    #[allow(dead_code)]
    pub const SCORE_NORMALIZE_JSON: &str = "reports/tooling/score_normalize.json";
    #[allow(dead_code)]
    pub const SCORE_NORMALIZE_MD: &str = "reports/tooling/score_normalize.md";
    #[allow(dead_code)]
    pub const PERF_ENGINEERING_REPORT_JSON: &str = "reports/tooling/perf_engineering_report.json";
    #[allow(dead_code)]
    pub const PERF_ENGINEERING_REPORT_MD: &str = "reports/tooling/perf_engineering_report.md";
    #[allow(dead_code)]
    pub const ABI_PERF_GATE_JSON: &str = "reports/tooling/abi_perf_gate.json";
    #[allow(dead_code)]
    pub const ABI_PERF_GATE_MD: &str = "reports/tooling/abi_perf_gate.md";
    #[allow(dead_code)]
    pub const RELEASE_NOTES_MD: &str = "reports/tooling/release_notes.md";
    #[allow(dead_code)]
    pub const RELEASE_MANIFEST_JSON: &str = "reports/tooling/release_manifest.json";
    #[allow(dead_code)]
    pub const RELEASE_MANIFEST_MD: &str = "reports/tooling/release_manifest.md";
    #[allow(dead_code)]
    pub const SUPPORT_DIAGNOSTICS_JSON: &str = "reports/tooling/support_diagnostics.json";
    #[allow(dead_code)]
    pub const SUPPORT_DIAGNOSTICS_MD: &str = "reports/tooling/support_diagnostics.md";
    #[allow(dead_code)]
    pub const LINUX_ABI_SEMANTIC_MATRIX_JSON: &str = "reports/linux_abi_semantic_matrix/summary.json";
    #[allow(dead_code)]
    pub const LINUX_ABI_SEMANTIC_MATRIX_MD: &str = "reports/linux_abi_semantic_matrix/summary.md";
    #[allow(dead_code)]
    pub const LINUX_ABI_SYSCALL_COVERAGE_ROWS_JSON: &str =
        "reports/linux_abi_semantic_matrix/syscall_rows.json";
    #[allow(dead_code)]
    pub const LINUX_ABI_UNSUPPORTED_DOC_JSON: &str =
        "reports/linux_abi_semantic_matrix/unsupported_syscalls.json";
    #[allow(dead_code)]
    pub const LINUX_ABI_UNSUPPORTED_DOC_MD: &str =
        "reports/linux_abi_semantic_matrix/unsupported_syscalls.md";
    #[allow(dead_code)]
    pub const LINUX_ABI_TREND_HISTORY_JSON: &str = "reports/linux_abi_trend/history.json";
    #[allow(dead_code)]
    pub const LINUX_ABI_TREND_DASHBOARD_JSON: &str = "reports/linux_abi_trend/dashboard.json";
    #[allow(dead_code)]
    pub const LINUX_ABI_TREND_DASHBOARD_MD: &str = "reports/linux_abi_trend/dashboard.md";
    #[allow(dead_code)]
    pub const LINUX_ABI_WORKLOAD_HISTORY_JSON: &str =
        "reports/linux_abi_workload_catalog/history.json";
    #[allow(dead_code)]
    pub const LINUX_ABI_WORKLOAD_CATALOG_JSON: &str =
        "reports/linux_abi_workload_catalog/catalog.json";
    #[allow(dead_code)]
    pub const LINUX_ABI_WORKLOAD_CATALOG_MD: &str =
        "reports/linux_abi_workload_catalog/catalog.md";
    #[allow(dead_code)]
    pub const LINUX_ABI_WORKLOAD_TREND_JSON: &str =
        "reports/linux_abi_workload_catalog/trend.json";
    #[allow(dead_code)]
    pub const LINUX_ABI_WORKLOAD_TREND_MD: &str =
        "reports/linux_abi_workload_catalog/trend.md";
    #[allow(dead_code)]
    pub const GLIBC_COMPAT_SPLIT_JSON: &str = "reports/glibc_compat_split/summary.json";
    #[allow(dead_code)]
    pub const GLIBC_COMPAT_SPLIT_MD: &str = "reports/glibc_compat_split/summary.md";
}
