pub mod build;
pub mod dashboard;
pub mod glibc;
pub mod linux;
pub mod release;
pub mod run;
pub mod runtime;
pub mod secureboot;
pub mod setup;
pub mod test;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

pub use crate::types::{Bootloader, ImageFormat};

pub use build::BuildAction;
pub use dashboard::DashboardAction;
pub use glibc::GlibcAction;
pub use linux::LinuxAbiAction;
pub use release::ReleaseAction;
pub use run::RunAction;
pub use runtime::AbSlotAction;
pub use secureboot::SecurebootAction;
pub use setup::SetupAction;
pub use test::TestAction;

/// The central automation tool for the Aether X OS pipeline.
/// Designed to streamline development, testing, image creation, and validation operations.
#[derive(Parser, Debug)]
#[command(name = "xtask")]
#[command(about = "Aether X OS Task Runner - Unified High-Performance Operations")]
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
