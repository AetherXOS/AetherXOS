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

use crate::utils::executable::Executable;
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

    /// Run xtask in non-interactive / CI mode. Equivalent to setting `XTASK_NONINTERACTIVE`.
    #[arg(long, global = true, default_value_t = false)]
    pub non_interactive: bool,

    /// Selected operational mode or isolated subsystem category.
    #[command(subcommand)]
    pub command: Commands,
}

macro_rules! define_commands {
    ($($variant:ident($action:ident) => $desc:expr),* $(; $($simple_variant:ident => $simple_desc:expr),*)?) => {
        #[derive(Subcommand, Debug)]
        pub enum Commands {
            $(
                #[doc = $desc]
                $variant {
                    #[command(subcommand)]
                    action: $action,
                },
            )*
            $(
                $(
                    #[doc = $simple_desc]
                    $simple_variant,
                )*
            )?
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
        }

        impl Executable for Commands {
            fn execute(&self) -> anyhow::Result<()> {
                use anyhow::Context;
                match self {
                    $(
                        Commands::$variant { action } => action.execute().context(concat!(stringify!($variant), " failure")),
                    )*
                    $(
                        $(
                            Commands::$simple_variant => {
                                match stringify!($simple_variant) {
                                    "CrashRecovery" => crate::commands::runtime::crash_recovery::execute().context("Crash recovery failure"),
                                    _ => Ok(())
                                }
                            }
                        )*
                    )?
                    Commands::CorePressure { words, lottery_words, format, out } => {
                        crate::commands::runtime::core_pressure::execute(words, lottery_words, format, out)
                            .context("Core pressure report failure")
                    }
                }
            }
        }
    }
}

define_commands! {
    Build(BuildAction) => "Infrastructure build operations",
    Run(RunAction) => "Emulation and deployment gateways",
    Test(TestAction) => "Validation suites",
    Setup(SetupAction) => "Host setup and bootstrapping",
    Dashboard(DashboardAction) => "Pipeline health visualization",
    LinuxAbi(LinuxAbiAction) => "Linux ABI compatibility",
    Secureboot(SecurebootAction) => "Secure Boot protocols",
    Release(ReleaseAction) => "Release engineering",
    AbSlot(AbSlotAction) => "A/B slot management",
    Glibc(GlibcAction) => "Glibc audit";
    CrashRecovery => "Panic diagnostics"
}
