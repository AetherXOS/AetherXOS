//! Shared configuration and constants for the xtask runner.
//! Centralizing these avoids magic values and duplication across modules.

pub const KERNEL_COMPAT_PATH: &str = "src/modules/linux_compat";
pub const KERNEL_SHIM_PATH: &str = "src/kernel/syscalls/linux_shim";
pub const SYSCALL_CONSTS_PATH: &str = "src/kernel/syscalls/syscalls_consts.rs";
pub const GENERATED_CONSTS_PATH: &str = "src/generated_consts.rs";

pub mod repo_paths {
    pub const ABI_GAP_SUMMARY: &str = "reports/abi_gap_inventory/summary.json";
    pub const ERRNO_CONFORMANCE_SUMMARY: &str = "reports/errno_conformance/summary.json";
    pub const SHIM_ERRNO_SUMMARY: &str = "reports/linux_shim_errno_conformance/summary.json";
    pub const SYSCALL_COVERAGE_SUMMARY: &str = "reports/syscall_coverage_summary.json";
    pub const ABI_READINESS_SUMMARY: &str = "reports/abi_readiness/summary.json";
    pub const POSIX_CONFORMANCE_SUMMARY: &str = "reports/posix_conformance/summary.json";
    pub const P_TIER_STATUS_JSON: &str = "reports/tooling/p_tier_status.json";
    pub const P_TIER_STATUS_MD: &str = "reports/tooling/p_tier_status.md";
}
