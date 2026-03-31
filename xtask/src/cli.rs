use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "AetherXOS Task Runner - Unified High-Performance Operations")]
#[command(
    long_about = "Replaces all legacy Python and PowerShell scripts with a single, \
    modular, type-safe Rust binary. Every OS workflow is accessible via subcommands."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build the OS kernel, boot image, and optionally the ISO
    Build {
        #[command(subcommand)]
        action: BuildAction,
    },

    /// Run the OS under an emulator (QEMU)
    Run {
        #[command(subcommand)]
        action: RunAction,
    },

    /// Test and validation commands
    Test {
        #[command(subcommand)]
        action: TestAction,
    },

    /// Dashboard management (build, test, open, agent)
    Dashboard {
        #[command(subcommand)]
        action: DashboardAction,
    },

    /// Workspace environment setup, audit, and repair
    Setup {
        #[command(subcommand)]
        action: SetupAction,
    },

    /// Secure Boot signing, validation, and OVMF matrix testing
    Secureboot {
        #[command(subcommand)]
        action: SecurebootAction,
    },

    /// Linux ABI conformance analysis and reports
    LinuxAbi {
        #[command(subcommand)]
        action: LinuxAbiAction,
    },

    /// glibc compatibility audit, closure tracking, and test framework
    Glibc {
        #[command(subcommand)]
        action: GlibcAction,
    },

    /// Release management (preflight, candidate gates, nightly)
    Release {
        #[command(subcommand)]
        action: ReleaseAction,
    },

    /// A/B boot slot management
    AbSlot {
        #[command(subcommand)]
        action: AbSlotAction,
    },

    /// Syscall coverage reporting and gate enforcement
    SyscallCoverage {
        /// Enable linux_compat feature gate evaluation
        #[arg(long)]
        linux_compat: bool,

        /// Output format
        #[arg(long, default_value = "md")]
        format: String,

        /// Output file path
        #[arg(long)]
        out: Option<String>,
    },

    /// Archive nightly artifacts
    ArchiveNightly {
        /// Custom run ID (defaults to timestamp)
        #[arg(long)]
        run_id: Option<String>,
    },

    /// QEMU soak/stress testing matrix
    SoakTest {
        /// Dry run mode
        #[arg(long)]
        dry_run: bool,
    },

    /// Crash-dump recovery pipeline (parse kernel logs, check monotonicity)
    CrashRecovery,

    /// Decode core pressure snapshot from raw syscall words
    CorePressure {
        /// Comma-separated usize words from GET_CORE_PRESSURE_SNAPSHOT (18 words)
        #[arg(long)]
        words: String,

        /// Optional comma-separated words from GET_LOTTERY_REPLAY_LATEST (5 words)
        #[arg(long)]
        lottery_replay_words: Option<String>,

        /// Output format (md or json)
        #[arg(long, default_value = "md")]
        format: String,

        /// Output file path
        #[arg(long)]
        out: Option<String>,
    },

    /// Generate P0/P1/P2 tier readiness status
    TierStatus,
}

// ---------------------------------------------------------------------------
// Build subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum BuildAction {
    /// Full build: compile kernel, generate initramfs, create ISO, run smoke test
    Full,
    /// Build only the bootable ISO image
    Iso,
    /// Build ISO with apt/userspace seed preloaded into initramfs
    AptIso {
        /// Base distro profile id from artifacts/tooling/installer/profiles.json
        #[arg(long, default_value = "debian")]
        profile: String,
        /// Optional comma-separated app targets from artifacts/tooling/installer/app_targets.json
        #[arg(long)]
        apps: Option<String>,
        /// Optional comma-separated package override list (replaces profile defaults)
        #[arg(long)]
        packages: Option<String>,
        /// Optional comma-separated package additions
        #[arg(long)]
        include: Option<String>,
        /// Optional comma-separated package removals
        #[arg(long)]
        exclude: Option<String>,
        /// Optional package mirror URL used by first-boot seed installer
        #[arg(long)]
        mirror: Option<String>,
    },
    /// Compile the kernel ELF binary only (no boot image assembly)
    Kernel,
    /// Generate initramfs archive from boot/initramfs directory
    Initramfs,
}

// ---------------------------------------------------------------------------
// Run subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum RunAction {
    /// Automated QEMU smoke test (headless, with timeout and panic detection)
    Smoke,
    /// Interactive QEMU session with display window
    Live,
}

// ---------------------------------------------------------------------------
// Test subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum TestAction {
    /// Run the full tooling quality gate
    QualityGate,
    /// Run host-side cargo check feature matrix validation
    Host {
        /// Run in release mode
        #[arg(long)]
        release: bool,
    },
    /// Run dashboard agent contract verification
    AgentContract,
    /// Run all core test tiers sequentially
    All {
        /// Use CI nextest profile and artifact-oriented behavior
        #[arg(long)]
        ci: bool,
    },
    /// POSIX conformance gate
    PosixConformance,
    /// Driver config smoke test
    DriverSmoke,
    /// Run a named CI tier locally or in GitHub Actions
    Tier {
        /// Tier name: fast, integration, nightly
        #[arg(long)]
        tier: String,
        /// Use CI nextest profile and artifact-oriented behavior
        #[arg(long)]
        ci: bool,
    },
    /// Linux app compatibility layered validator (strict/quick/qemu)
    LinuxAppCompat {
        /// Desktop smoke profile (enforces Wayland/X11 probes)
        #[arg(long)]
        desktop_smoke: bool,
        /// Quick mode
        #[arg(long)]
        quick: bool,
        /// Enable QEMU gate
        #[arg(long)]
        qemu: bool,
        /// Strict profile (enables CI + QEMU defaults)
        #[arg(long)]
        strict: bool,
        /// CI enforcement mode
        #[arg(long)]
        ci: bool,
        /// Require BusyBox probes to exist
        #[arg(long)]
        require_busybox: bool,
        /// Require glibc probes to exist
        #[arg(long)]
        require_glibc: bool,
        /// Require Wayland userspace graphics probes to pass
        #[arg(long)]
        require_wayland: bool,
        /// Require X11 userspace graphics probes to pass
        #[arg(long)]
        require_x11: bool,
        /// Require non-RamFS filesystem stack probes to pass
        #[arg(long)]
        require_fs_stack: bool,
        /// Require Linux package install stack probes to pass
        #[arg(long)]
        require_package_stack: bool,
        /// Require desktop application stack probes (XFCE/GNOME/Flutter prerequisites)
        #[arg(long)]
        require_desktop_app_stack: bool,
    },
    /// Kernel structural audit (long files, magic values, deep coupling)
    KernelRefactorAudit {
        /// Line-count threshold for long file hotspot reporting
        #[arg(long, default_value_t = 1200)]
        max_lines: usize,
        /// Minimum times a literal appears in a file to flag as magic-value candidate
        #[arg(long, default_value_t = 8)]
        magic_repeat_threshold: usize,
    },
}

// ---------------------------------------------------------------------------
// Dashboard subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum DashboardAction {
    /// Build dashboard data, HTML, and UI assets
    Build,
    /// Run dashboard unit and E2E tests
    Test,
    /// Open dashboard in browser
    Open,
    /// Start dashboard agent in background
    AgentStart {
        /// Skip auth checks (local lab use only)
        #[arg(long)]
        no_safe: bool,
    },
}

// ---------------------------------------------------------------------------
// Setup subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum SetupAction {
    /// Audit the host environment for missing dependencies
    Audit,
    /// Auto-repair missing host dependencies
    Repair,
    /// Full workspace bootstrap (audit + repair + build + smoke + dashboard)
    Bootstrap,
    /// Bootstrap a Debian rootfs, package Flutter, install into rootfs, make image and boot QEMU
    BootstrapFlutter {
        /// Output directory for the rootfs (host path)
        #[arg(long)]
        outdir: String,

        /// Flutter tar.xz URL to download
        #[arg(long)]
        flutter_url: String,

        /// Kernel image to boot (defaults to kernel.x in repo root)
        #[arg(long, default_value = "kernel.x")]
        kernel_image: String,
    },
}

// ---------------------------------------------------------------------------
// Secure Boot subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum SecurebootAction {
    /// Sign EFI binaries with sbsign/pesign
    Sign {
        /// Dry run (copy without signing)
        #[arg(long)]
        dry_run: bool,
        /// Fail if signature verification tool/output is unavailable or invalid
        #[arg(long)]
        strict_verify: bool,
    },
    /// Validate SBAT metadata presence in EFI binaries
    SbatValidate {
        /// Strict mode (non-zero exit on failure)
        #[arg(long)]
        strict: bool,
    },
    /// Generate TPM PCR / event-log summary report
    PcrReport,
    /// Generate MOK enrollment runbook
    MokPlan,
    /// Run OVMF Secure Boot matrix smoke test
    OvmfMatrix {
        /// Dry run mode
        #[arg(long)]
        dry_run: bool,
    },
}

// ---------------------------------------------------------------------------
// Linux ABI subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum LinuxAbiAction {
    /// Linux ABI gap inventory analysis
    GapInventory,
    /// Linux ABI readiness score
    ReadinessScore,
    /// errno conformance check
    ErrnoConformance,
    /// Shim errno conformance
    ShimErrnoConformance,
    /// Linux platform readiness report
    PlatformReadiness,
    /// Linux desktop integration plan (Wayland/X11 + XFCE/GNOME roadmap)
    DesktopPlan,
    /// ABI gate enforcement
    Gate,
    /// Policy drift ABI smoke test
    PolicyDrift,
    /// glibc needs analysis
    GlibcNeeds,
    /// P-tier status report
    PTierStatus,
    /// P2 gap report
    P2GapReport,
    /// P2 gap gate
    P2GapGate,
}

// ---------------------------------------------------------------------------
// glibc Completeness subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum GlibcAction {
    /// Audit all 50 critical glibc syscalls for completeness, tests, and blockers
    Audit {
        /// Output format (md, json, csv)
        #[arg(long, default_value = "md")]
        format: String,

        /// Output file path (defaults to stdout)
        #[arg(long)]
        out: Option<String>,

        /// Include implementation source code snippets
        #[arg(long)]
        verbose: bool,
    },
    /// Run comprehensive glibc closure test suite (blockers, edge cases)
    ClosureGate {
        /// Quick mode (high-priority syscalls only)
        #[arg(long)]
        quick: bool,

        /// Strict mode (CI-grade enforcement)
        #[arg(long)]
        strict: bool,

        /// Focus on specific syscall family (file_io, process, memory, signals, threading)
        #[arg(long)]
        family: Option<String>,

        /// Output format (md, json, csv)
        #[arg(long, default_value = "md")]
        format: String,

        /// Output file path
        #[arg(long)]
        out: Option<String>,
    },
    /// Generate glibc completeness scorecard (visual dashboard integration)
    Scorecard {
        /// Output format (md, json)
        #[arg(long, default_value = "json")]
        format: String,

        /// Output file path
        #[arg(long)]
        out: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Release subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum ReleaseAction {
    /// Run full release preflight validation
    Preflight {
        /// Skip host tests
        #[arg(long)]
        skip_host_tests: bool,
        /// Skip boot artifact generation
        #[arg(long)]
        skip_boot_artifacts: bool,
    },
    /// Release candidate gate
    CandidateGate,
    /// P0 readiness gate
    P0Gate,
    /// P0 release acceptance
    P0Acceptance,
    /// P1 nightly run
    P1Nightly,
    /// P1 release acceptance
    P1Acceptance,
    /// Combined P0+P1 nightly pipeline
    P0P1Nightly,
}

// ---------------------------------------------------------------------------
// A/B Slot subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum AbSlotAction {
    /// Initialize A/B slot metadata
    Init,
    /// Stage artifacts to a slot
    Stage {
        /// Target slot (A or B)
        #[arg(long)]
        slot: String,
    },
    /// Nightly slot flip
    NightlyFlip,
    /// Boot recovery gate
    RecoveryGate,
}
