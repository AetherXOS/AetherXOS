//! Centralized constants for xtask commands.
//! Eliminates hardcoded strings across commands and improves maintainability.

// ============================================================================
// Architecture-related Constants
// ============================================================================

/// Supported architectures for kernel builds
pub mod arch {
    pub const X86_64: &str = "x86_64";
    pub const AARCH64: &str = "aarch64";
    
    /// Map architecture names to target triples for bare-metal builds
        pub fn to_bare_metal_triple(arch: &str) -> Option<&'static str> {
        match arch {
            X86_64 => Some("x86_64-unknown-none"),
            AARCH64 => Some("aarch64-unknown-none"),
            _ => None,
        }
    }
    
        /// Validate architecture is supported
        pub fn is_valid(arch: &str) -> bool {
            matches!(arch, X86_64 | AARCH64)
        }
    
        /// Get all supported architectures
        pub fn supported() -> &'static [&'static str] {
            &[X86_64, AARCH64]
        }
}

// ============================================================================
// Cargo-related Constants
// ============================================================================

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

// ============================================================================
// NPM/Dashboard Constants
// ============================================================================

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

// ============================================================================
// System Commands (Platform-specific)
// ============================================================================

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

// ============================================================================
// Tool Names and Paths
// ============================================================================

pub mod tools {
    pub const QEMU_X86_64: &str = "qemu-system-x86_64";
    pub const QEMU_X86_64_EXE: &str = "qemu-system-x86_64.exe";
    pub const XORRISO: &str = "xorriso";
    pub const RUSTC: &str = "rustc";
    pub const CARGO: &str = "cargo";
}

// ============================================================================
// Directory and File Paths
// ============================================================================

pub mod paths {
    pub const BOOT_INITRAMFS: &str = "boot/initramfs";
    pub const ARTIFACTS_BOOT_IMAGE: &str = "artifacts/boot_image";
    pub const ARTIFACTS_BOOT_IMAGE_STAGE: &str = "artifacts/boot_image/stage";
    pub const BOOT_INITRAMFS_OUT: &str = "artifacts/boot_image/stage/boot/initramfs.cpio.gz";
    pub const DASHBOARD_DIR: &str = "dashboard";
}

// ============================================================================
// Kernel Configuration Constants
// ============================================================================

pub mod kernel {
    pub const SECTOR_SIZE_DEFAULT: usize = 512;
}

// ============================================================================
// Telemetry and Mount Policy Constants
// ============================================================================

pub mod telemetry {
    pub const MOUNT_POLICY_PATH: &str = "/run/hypercore/telemetry/mount_policy_events";
    pub const EVENT_TMPFS_FALLBACK: &str = "event=tmpfs_fallback";
    pub const EVENT_DISKFS_MOUNTED: &str = "event=diskfs_mounted";
    pub const EVENT_DISKFS_MODE_SET: &str = "event=diskfs_mode_set";
}

// ============================================================================
// Test Tier and Feature Constants
// ============================================================================

pub mod test {
    pub const TIER_FAST: &str = "fast";
    pub const TIER_INTEGRATION: &str = "integration";
    pub const TIER_NIGHTLY: &str = "nightly";
    
    pub fn is_valid_tier(tier: &str) -> bool {
        matches!(tier, TIER_FAST | TIER_INTEGRATION | TIER_NIGHTLY)
    }
    
    pub fn all_tiers() -> &'static [&'static str] {
        &[TIER_FAST, TIER_INTEGRATION, TIER_NIGHTLY]
    }
    
    /// Features needed for comprehensive kernel testing
    pub const TEST_FEATURES: &str = "kernel_test_mode,vfs,drivers";
}

// ============================================================================
// Default Configurations
// ============================================================================

pub mod defaults {
    /// Default architecture for single-arch builds
    pub const ARCH: &str = "x86_64";
    
    /// Get cargo profile string (debug or release)
    pub fn profile_name(is_release: bool) -> &'static str {
        if is_release {
            PROFILE_RELEASE
        } else {
            PROFILE_DEBUG
        }
    }
    
    /// Default flutter channel for engine seeds
    pub const FLUTTER_CHANNEL: &str = "stable";
    
    /// Default profile for builds (debug/release)
    pub const PROFILE_DEBUG: &str = "debug";
    pub const PROFILE_RELEASE: &str = "release";
}
