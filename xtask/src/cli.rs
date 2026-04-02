use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// The central automation tool for the AetherXOS pipeline.
/// Designed to streamline development, testing, image creation, and validation operations.
#[derive(Parser, Debug)]
#[command(name = "xtask")]
#[command(about = "AetherXOS Task Runner - Unified High-Performance Operations")]
#[command(
    long_about = "Replaces all legacy scripts with a single, modular, type-safe Rust binary. \
    Every OS workflow is dynamically accessible via subcommands."
)]
pub struct Cli {
    /// Global output directory for generated artifacts and images.
    #[arg(long, global = true, default_value = "artifacts")]
    pub outdir: PathBuf,

    /// Selected operational mode or isolated subsystem category.
    #[command(subcommand)]
    pub command: Commands,
}

/// Identifies the bootloader integration protocol to inject into the storage medium.
#[derive(Clone, Debug, ValueEnum, Default)]
pub enum Bootloader {
    /// Limine Advanced Boot Protocol
    #[default]
    Limine,
    /// Multiboot2 Specification Standard
    Multiboot2,
    /// Legacy GRUB Payload Standard
    Grub,
    /// Kernel executed identically to a standard EFI binary, sans external bootloader.
    Direct,
}

/// Identifies the desired final package storage format for the assembled operating system.
#[derive(Clone, Debug, ValueEnum, Default)]
pub enum ImageFormat {
    /// Bootable CD/DVD ISO9660 image (ISOHybrid capable).
    #[default]
    Iso,
    /// Raw dd-flashable disk image, ideal for USB drives and physical SD cards.
    Img,
    /// Virtual Hard Disk target wrapper natively utilized by Hyper-V and VirtualBox.
    Vhd,
}

/// Organizational hierarchies representing independent Xtask subsystems.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Infrastructure build operations (Compile kernel, pack rootfs, construct full bootable images)
    Build {
        #[command(subcommand)]
        action: BuildAction,
    },

    /// Emulation gateways and direct runtime deployment targets
    Run {
        #[command(subcommand)]
        action: RunAction,
    },

    /// Comprehensive kernel logic checks, UI assertions, and tooling validations suites
    Test {
        #[command(subcommand)]
        action: TestAction,
    },

    /// Configuration, bootstrapping, host environmental gap remediation
    Setup {
        #[command(subcommand)]
        action: SetupAction,
    },

    /// Status reporting, CI/CD telemetry aggregations, overview metrics
    Dashboard {
        #[command(subcommand)]
        action: DashboardAction,
    },

    /// Linux application compatibility parsing, coverage auditing, bridging metrics
    LinuxAbi {
        #[command(subcommand)]
        action: LinuxAbiAction,
    },

    /// Cryptographic signing routines, SBAT validations, Platform Configuration Registers logic
    Secureboot {
        #[command(subcommand)]
        action: SecurebootAction,
    },

    /// Release engineering, preflight gates, and candidate acceptance
    Release {
        #[command(subcommand)]
        action: ReleaseAction,
    },

    /// Runtime A/B slot management and boot recovery gates
    AbSlot {
        #[command(subcommand)]
        action: AbSlotAction,
    },

    CorePressure {
        #[arg(long)]
        words: String,

        #[arg(long)]
        lottery_words: Option<String>,

        #[arg(long, default_value = "text")]
        format: String,

        #[arg(long)]
        out: Option<String>,
    },

    CrashRecovery,

    Glibc {
        #[command(subcommand)]
        action: GlibcAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum BuildAction {
    /// Integrates OS elements (Kernel + RootFS) into an immediately bootable payload target
    Full {
        /// Explicit Host/Guest compiler target architecture (e.g., x86_64, aarch64)
        #[arg(long, default_value = "x86_64")]
        arch: String,

        /// Assigned boot target application protocol for OS handoff
        #[arg(long, default_value = "limine")]
        bootloader: Bootloader,

        /// Format boundary wrapper generated for virtualization or raw deployment
        #[arg(long, default_value = "iso")]
        format: ImageFormat,

        /// Toggle LLVM/Rust optimization profiles flag
        #[arg(long)]
        release: bool,
    },

    /// Aggregates existing pre-built objects strictly for Image wrapper creation
    Image {
        #[arg(long, default_value = "limine")]
        bootloader: Bootloader,

        #[arg(long, default_value = "iso")]
        format: ImageFormat,
    },

    /// Instructs the compiler to strictly compile the Kernel ELF void of external wrappers
    Kernel {
        #[arg(long, default_value = "x86_64")]
        arch: String,

        #[arg(long)]
        release: bool,
    },

    /// Archives core userspace modules into the pre-mount Initial RAM filesystem layout
    Initramfs,

    /// Compiles a dedicated userspace program and bundles it directly into the initramfs layout
    App {
        /// The exact package/crate name of the userspace application to construct
        name: String,

        /// Toggle LLVM/Rust optimization profiles flag
        #[arg(long)]
        release: bool,
    },

    /// Generate P0/P1/P2 tier readiness status
    TierStatus,
}

#[derive(Subcommand, Debug)]
pub enum RunAction {
    /// Execute robust QEMU pipeline targeting automated timeout evaluation loops
    Smoke {
        #[arg(long, default_value = "limine")]
        bootloader: Bootloader,
    },
    /// Provide graphical interactive emulator access allowing user UI validations
    Live {
        #[arg(long, default_value = "uefi")]
        firmware: String, // Accepts 'bios' or 'uefi' overrides
    },
    /// Immediately stream compiled artifacts via block operations natively to an assigned storage drive
    BareMetalDeploy {
        #[arg(long)]
        device: String, // Explicit target mountpoint e.g., /dev/sdb or \\.\PhysicalDrive1
    },
    /// Launches QEMU in suspended execution mode and spawns a connected GDB instance automatically
    Debug {
        #[arg(long, default_value = "uefi")]
        firmware: String,
    },
    /// Launches an ephemeral local network server facilitating PXE network booting for physical testing
    PxeServer {
        #[arg(long, default_value = "69")]
        port: u16,
    },
}

#[derive(Subcommand, Debug)]
pub enum TestAction {
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
        tier: String,
        /// Use CI nextest profile and artifact-oriented behavior
        #[arg(long)]
        ci: bool,
    },
    /// Linux app compatibility layered validator (strict/quick/qemu)
    LinuxAppCompat {
        #[arg(long)]
        desktop_smoke: bool,
        #[arg(long)]
        quick: bool,
        #[arg(long)]
        qemu: bool,
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        ci: bool,
        #[arg(long)]
        require_busybox: bool,
        #[arg(long)]
        require_glibc: bool,
        #[arg(long)]
        require_wayland: bool,
        #[arg(long)]
        require_x11: bool,
        #[arg(long)]
        require_fs_stack: bool,
        #[arg(long)]
        require_package_stack: bool,
        #[arg(long)]
        require_desktop_app_stack: bool,
    },
    /// Audit kernel source for refactoring candidate areas
    KernelRefactorAudit {
        #[arg(long, default_value = "500")]
        max_lines: usize,
        #[arg(long, default_value = "3")]
        magic_repeat_threshold: usize,
    },
}

#[derive(Subcommand, Debug)]
pub enum SetupAction {
    Audit,
    Repair,
    Bootstrap,
    /// Synchronize and provision external vendored bootloaders (e.g., pulling Limine from GitHub releases)
    FetchBootloader,
    /// Provision target compilers required for host cross-compilation boundaries
    Toolchain,
}

#[derive(Subcommand, Debug)]
pub enum DashboardAction {
    Build,
    Test,
    Open,
    AgentStart {
        #[arg(long)]
        no_safe: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum LinuxAbiAction {
    GapInventory,
    Gate,
    ErrnoConformance,
    ShimErrnoConformance,
    ReadinessScore,
    P2GapReport,
    P2GapGate,
}

#[derive(Subcommand, Debug)]
pub enum SecurebootAction {
    Sign {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        strict_verify: bool,
    },
    SbatValidate {
        #[arg(long)]
        strict: bool,
    },
    PcrReport,
    MokPlan,
    OvmfMatrix {
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum ReleaseAction {
    Preflight {
        #[arg(long)]
        skip_host_tests: bool,
        #[arg(long)]
        skip_boot_artifacts: bool,
    },
    CandidateGate,
    P0Gate,
    P0Acceptance,
    P1Nightly,
    P1Acceptance,
    P0P1Nightly,
}

#[derive(Subcommand, Debug)]
pub enum AbSlotAction {
    Init,
    Stage { slot: String },
    NightlyFlip,
    RecoveryGate,
}

#[derive(Subcommand, Debug)]
pub enum GlibcAction {
    Audit {
        #[arg(long, default_value = "md")]
        format: String,
        #[arg(long)]
        out: Option<String>,
        #[arg(long)]
        verbose: bool,
    },
    ClosureGate {
        #[arg(long)]
        quick: bool,
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        family: Option<String>,
        #[arg(long, default_value = "md")]
        format: String,
        #[arg(long)]
        out: Option<String>,
    },
    Scorecard {
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long)]
        out: Option<String>,
    },
}
