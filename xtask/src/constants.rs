pub mod arch {
    pub const X86_64: &str = "x86_64";
    pub const AARCH64: &str = "aarch64";

    pub fn to_bare_metal_triple(arch: &str) -> Option<&'static str> {
        match arch {
            X86_64 => Some("x86_64-unknown-none"),
            AARCH64 => Some("aarch64-unknown-none"),
            _ => None,
        }
    }

    pub fn is_valid(arch: &str) -> bool {
        matches!(arch, X86_64 | AARCH64)
    }

    pub fn supported() -> &'static [&'static str] {
        &[X86_64, AARCH64]
    }
}

pub mod cargo {
    pub const CMD_BUILD: &str = "build";
    pub const CMD_CHECK: &str = "check";
    pub const CMD_TEST: &str = "test";
    pub const CMD_RUN: &str = "run";
    pub const ARG_TARGET: &str = "--target";
    pub const ARG_RELEASE: &str = "--release";
    pub const ARG_MANIFEST_PATH: &str = "--manifest-path";
    pub const ARG_FEATURES: &str = "--features";
    pub const MANIFEST_FILE: &str = "Cargo.toml";
}

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

pub mod tools {
    pub const QEMU_X86_64: &str = "qemu-system-x86_64";
    pub const QEMU_X86_64_EXE: &str = "qemu-system-x86_64.exe";
    pub const XORRISO: &str = "xorriso";
    pub const RUSTC: &str = "rustc";
    pub const CARGO: &str = "cargo";
}

pub mod paths {
    pub const BOOT_INITRAMFS: &str = "boot/initramfs";
    pub const ARTIFACTS_BOOT_IMAGE: &str = "artifacts/boot_image";
    pub const ARTIFACTS_BOOT_IMAGE_STAGE: &str = "artifacts/boot_image/stage";
    pub const BOOT_INITRAMFS_OUT: &str = "artifacts/boot_image/stage/boot/initramfs.cpio.gz";
    pub const DASHBOARD_DIR: &str = "dashboard";
}

pub mod telemetry {
    pub const EVENT_TMPFS_FALLBACK: &str = "event=tmpfs_fallback";
    pub const EVENT_DISKFS_MOUNTED: &str = "event=diskfs_mounted";
    pub const EVENT_DISKFS_MODE_SET: &str = "event=diskfs_mode_set";
}

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

    pub const TEST_FEATURES: &str = "kernel_test_mode,vfs,drivers";
}

pub mod defaults {
    pub const ARCH: &str = "x86_64";
    const PROFILE_DEBUG: &str = "debug";
    const PROFILE_RELEASE: &str = "release";

    pub fn profile_name(is_release: bool) -> &'static str {
        if is_release {
            PROFILE_RELEASE
        } else {
            PROFILE_DEBUG
        }
    }
}
