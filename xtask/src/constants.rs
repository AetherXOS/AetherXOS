//! Centralized constants for xtask commands.
//! Keeps paths, defaults, and tool names in one place so command modules stay thin.

use std::path::PathBuf;

use crate::utils::paths as fs_paths;

/// Cargo-related constants.
#[allow(dead_code)]
pub mod cargo {
    pub const CMD_BUILD: &str = "build";
    pub const CMD_CHECK: &str = "check";
    pub const CMD_TEST: &str = "test";
    pub const CMD_RUN: &str = "run";

    pub const ARG_TARGET: &str = "--target";
    pub const ARG_RELEASE: &str = "--release";
    pub const ARG_MANIFEST_PATH: &str = "--manifest-path";
    pub const ARG_FEATURES: &str = "--features";
    pub const ARG_NO_DEFAULT_FEATURES: &str = "--no-default-features";
    pub const ARG_ALL_FEATURES: &str = "--all-features";
    pub const ARG_WORKSPACE: &str = "--workspace";

    pub const MANIFEST_FILE: &str = "Cargo.toml";
}

/// NPM/Dashboard constants.
pub mod npm {
    pub const ARG_RUN: &str = "run";
    pub const ARG_HOST: &str = "--host";
    pub const ARG_SEPARATOR: &str = "--";
    pub const ARG_TEST_RUN: &str = "--run";

    pub const SCRIPT_BUILD: &str = "build";
    pub const SCRIPT_CHECK: &str = "check";
    pub const SCRIPT_DEV: &str = "dev";
    pub const SCRIPT_TEST_UNIT: &str = "test:unit";
    pub const SCRIPT_WORKFLOW_TEST: &str = "test:workflow";

    pub const HOST_SAFE: &str = "127.0.0.1";
    pub const HOST_UNSAFE: &str = "0.0.0.0";

    pub const BUILD_OUTPUT_PATH: &str = "dist/index.html";
}

/// Platform-specific commands.
pub mod commands {
    pub mod windows {
        pub const CMD_SHELL: &str = "cmd";
        pub const CMD_FLAG: &str = "/C";
        pub const CMD_START: &str = "start";
    }

    pub mod unix {
        pub const CMD_OPEN: &str = "xdg-open";
    }
}

/// Tool names and external binaries.
#[allow(dead_code)]
pub mod tools {
    pub const QEMU_X86_64: &str = "qemu-system-x86_64";
    pub const QEMU_X86_64_EXE: &str = "qemu-system-x86_64.exe";
    pub const QEMU_IMG: &str = "qemu-img";
    pub const QEMU_IMG_EXE: &str = "qemu-img.exe";
    pub const XORRISO: &str = "xorriso";
    pub const XORRISO_EXE: &str = "xorriso.exe";
    pub const RUSTC: &str = "rustc";
    pub const CARGO: &str = "cargo";
}

/// Repository-relative and staged artifact paths.
#[allow(dead_code)]
pub mod paths {
    use super::*;

    pub const ARTIFACTS_DIR: &str = "artifacts";
    pub const BOOT_INITRAMFS_SRC: &str = "boot/initramfs";
    pub const BOOT_IMAGE_ROOT: &str = "artifacts/boot_image";
    pub const BOOT_IMAGE_STAGE: &str = "artifacts/boot_image/stage";
    pub const BOOT_IMAGE_STAGE_BOOT: &str = "artifacts/boot_image/stage/boot";
    pub const BOOT_IMAGE_STAGE_KERNEL: &str = "artifacts/boot_image/stage/boot/aethercore.elf";
    pub const BOOT_IMAGE_STAGE_INITRAMFS: &str = "artifacts/boot_image/stage/boot/initramfs.cpio.gz";
    pub const BOOT_IMAGE_STAGE_LIMINE: &str = "artifacts/boot_image/stage/boot/limine.conf";
    pub const BOOT_AB_ROOT: &str = "artifacts/boot_ab";
    pub const BOOT_AB_STATE: &str = "artifacts/boot_ab/state.json";
    pub const BOOT_IMAGE_ISO_ROOT: &str = "artifacts/boot_image/iso_root";
    pub const DASHBOARD_DIR: &str = "dashboard";
    pub const HOST_TOOLS_BIN: &str = "artifacts/host_tools/bin";
    pub const LIMINE_BIN_DIR: &str = "artifacts/limine/bin";
    pub const CRASH_LOGS_DIR: &str = "artifacts/crash";
    pub const CRASH_REPORTS_DIR: &str = "reports/crash_pipeline";
    pub const KERNEL_REFACTOR_AUDIT_DIR: &str = "reports/kernel_refactor_audit";
    pub const QEMU_SMOKE_LOG: &str = "artifacts/boot_image/qemu_smoke.log";
    pub const QEMU_SMOKE_JUNIT: &str = "artifacts/qemu_smoke_junit.xml";
    pub const QEMU_SMOKE_JSON: &str = "artifacts/qemu_smoke_summary.json";
    pub const QEMU_SOAK_ROOT: &str = "artifacts/qemu_soak";
    pub const REPORTS_AB_BOOT_RECOVERY_GATE: &str = "reports/ab_boot_recovery_gate";
    pub const SYSCALL_COVERAGE_SUMMARY: &str = "reports/syscall_coverage_summary.json";
    pub const SECUREBOOT_SIGNED_DIR: &str = "artifacts/secureboot/signed";
    pub const SECUREBOOT_SIGN_REPORT: &str = "reports/secureboot/sign_report.json";
    pub const SECUREBOOT_SBAT_REPORT: &str = "reports/secureboot/sbat_report.json";
    pub const SECUREBOOT_PCR_REPORT: &str = "reports/secureboot/pcr_report.json";
    pub const SECUREBOOT_OVMF_MATRIX_DIR: &str = "reports/secureboot/ovmf_matrix";
    pub const OVMF_DIR: &str = "artifacts/ovmf";
    pub const SECUREBOOT_ROOT: &str = "artifacts/secureboot";

    pub fn artifact_dir() -> PathBuf {
        fs_paths::resolve(ARTIFACTS_DIR)
    }

    pub fn boot_initramfs_src() -> PathBuf {
        fs_paths::resolve(BOOT_INITRAMFS_SRC)
    }

    pub fn boot_image_stage_boot() -> PathBuf {
        fs_paths::resolve(BOOT_IMAGE_STAGE_BOOT)
    }

    pub fn boot_image_stage_kernel() -> PathBuf {
        fs_paths::resolve(BOOT_IMAGE_STAGE_KERNEL)
    }

    pub fn boot_image_stage_initramfs() -> PathBuf {
        fs_paths::resolve(BOOT_IMAGE_STAGE_INITRAMFS)
    }

    pub fn boot_image_stage_limine() -> PathBuf {
        fs_paths::resolve(BOOT_IMAGE_STAGE_LIMINE)
    }

    pub fn boot_ab_root() -> PathBuf {
        fs_paths::resolve(BOOT_AB_ROOT)
    }

    pub fn boot_ab_state() -> PathBuf {
        fs_paths::resolve(BOOT_AB_STATE)
    }

    pub fn boot_image_iso_root() -> PathBuf {
        fs_paths::resolve(BOOT_IMAGE_ISO_ROOT)
    }

    pub fn dashboard_dir() -> PathBuf {
        fs_paths::resolve(DASHBOARD_DIR)
    }

    pub fn host_tools_bin() -> PathBuf {
        fs_paths::resolve(HOST_TOOLS_BIN)
    }

    pub fn limine_bin_dir() -> PathBuf {
        fs_paths::resolve(LIMINE_BIN_DIR)
    }

    pub fn crash_logs_dir() -> PathBuf {
        fs_paths::resolve(CRASH_LOGS_DIR)
    }

    pub fn crash_reports_dir() -> PathBuf {
        fs_paths::resolve(CRASH_REPORTS_DIR)
    }

    pub fn kernel_refactor_audit_dir() -> PathBuf {
        fs_paths::resolve(KERNEL_REFACTOR_AUDIT_DIR)
    }

    pub fn qemu_smoke_log() -> PathBuf {
        fs_paths::resolve(QEMU_SMOKE_LOG)
    }

    pub fn qemu_smoke_junit() -> PathBuf {
        fs_paths::resolve(QEMU_SMOKE_JUNIT)
    }

    pub fn qemu_smoke_json() -> PathBuf {
        fs_paths::resolve(QEMU_SMOKE_JSON)
    }

    pub fn qemu_soak_root() -> PathBuf {
        fs_paths::resolve(QEMU_SOAK_ROOT)
    }

    pub fn reports_ab_boot_recovery_gate() -> PathBuf {
        fs_paths::resolve(REPORTS_AB_BOOT_RECOVERY_GATE)
    }

    pub fn syscall_coverage_summary() -> PathBuf {
        fs_paths::resolve(SYSCALL_COVERAGE_SUMMARY)
    }

    pub fn secureboot_signed_dir() -> PathBuf {
        fs_paths::resolve(SECUREBOOT_SIGNED_DIR)
    }

    pub fn secureboot_sign_report() -> PathBuf {
        fs_paths::resolve(SECUREBOOT_SIGN_REPORT)
    }

    pub fn secureboot_sbat_report() -> PathBuf {
        fs_paths::resolve(SECUREBOOT_SBAT_REPORT)
    }

    pub fn secureboot_pcr_report() -> PathBuf {
        fs_paths::resolve(SECUREBOOT_PCR_REPORT)
    }

    pub fn secureboot_ovmf_matrix_dir() -> PathBuf {
        fs_paths::resolve(SECUREBOOT_OVMF_MATRIX_DIR)
    }

    pub fn ovmf_dir() -> PathBuf {
        fs_paths::resolve(OVMF_DIR)
    }

    pub fn secureboot_root() -> PathBuf {
        fs_paths::resolve(SECUREBOOT_ROOT)
    }
}

/// Kernel configuration constants.
#[allow(dead_code)]
pub mod kernel {
    pub const SECTOR_SIZE_DEFAULT: usize = 512;
}

/// Telemetry and mount policy constants.
#[allow(dead_code)]
pub mod telemetry {
    pub const MOUNT_POLICY_PATH: &str = "/run/aethercore/telemetry/mount_policy_events";
    pub const EVENT_TMPFS_FALLBACK: &str = "event=tmpfs_fallback";
    pub const EVENT_DISKFS_MOUNTED: &str = "event=diskfs_mounted";
    pub const EVENT_DISKFS_MODE_SET: &str = "event=diskfs_mode_set";
}

/// Test tier and feature constants.
#[allow(dead_code)]
pub mod test {
    pub const TIER_FAST: &str = "fast";
    pub const TIER_INTEGRATION: &str = "integration";
    pub const TIER_NIGHTLY: &str = "nightly";

    /// Features needed for comprehensive kernel testing.
    pub const TEST_FEATURES: &str = "kernel_test_mode,vfs,drivers";

    pub fn is_valid_tier(tier: &str) -> bool {
        matches!(tier, TIER_FAST | TIER_INTEGRATION | TIER_NIGHTLY)
    }

    pub fn all_tiers() -> &'static [&'static str] {
        &[TIER_FAST, TIER_INTEGRATION, TIER_NIGHTLY]
    }
}

/// Default configurations used by CLI and command orchestration.
#[allow(dead_code)]
pub mod defaults {
    pub mod build {
        pub const ARCH: aethercore_common::TargetArch = aethercore_common::TargetArch::X86_64;
        pub const BOOTLOADER: &str = "limine";
        pub const FORMAT: &str = "iso";
        pub const USERSPACE_TARGET: &str = aethercore_common::TargetArch::X86_64.to_bare_metal_triple();
    }

    pub mod run {
        pub const FIRMWARE: &str = "uefi";
        pub const MEMORY_MB: u32 = 512;
        pub const SMP_CORES: u32 = 2;
        pub const PXE_PORT: u16 = 69;
        pub const KERNEL_APPEND: &str = "console=ttyS0 loglevel=7";
        pub const QEMU_SMOKE_TIMEOUT_SEC: u64 = 20;
        pub const WAIT_POLL_INTERVAL_MS: u64 = 100;
    }

    pub mod audit {
        pub const MAX_LINES: usize = 500;
        pub const MAGIC_REPEAT_THRESHOLD: usize = 3;
    }

    pub mod ab_slot {
        pub const MAX_CONSECUTIVE_FAILURES: u32 = 3;
    }

    pub mod glibc {
        pub const FORMAT_MD: &str = "md";
        pub const FORMAT_JSON: &str = "json";
    }

    pub mod flutter {
        pub const CHANNEL: &str = "stable";
    }

    pub mod profile {
        pub const DEBUG: &str = "debug";
        pub const RELEASE: &str = "release";

        pub fn name(is_release: bool) -> &'static str {
            if is_release {
                RELEASE
            } else {
                DEBUG
            }
        }
    }
}
