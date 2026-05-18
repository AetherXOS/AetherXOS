pub mod build;
pub mod clean;
pub mod completion;
pub mod dashboard;
pub mod glibc;
pub mod interactive;
pub mod linux;
pub mod pipeline;
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

pub use build::{BuildAction, CommonBuildArgs};
pub use clean::CleanAction;
pub use completion::CompletionAction;
pub use dashboard::DashboardAction;
pub use glibc::GlibcAction;
pub use interactive::InteractiveAction;
pub use linux::LinuxAbiAction;
pub use pipeline::PipelineAction;
pub use release::ReleaseAction;
pub use run::RunAction;
pub use runtime::AbSlotAction;
pub use secureboot::SecurebootAction;
pub use setup::SetupAction;
pub use test::TestAction;

/// The central automation tool for the Aether X OS pipeline.
#[derive(Parser, Debug)]
#[command(name = "xtask")]
#[command(about = "Aether X OS Task Runner - Unified High-Performance Operations")]
pub struct Cli {
    /// Global output directory for generated artifacts and images.
    #[arg(long, global = true, default_value = "artifacts")]
    pub outdir: PathBuf,

    /// Run xtask in non-interactive / CI mode. Equivalent to setting `XTASK_NONINTERACTIVE`.
    #[arg(long, global = true, default_value_t = false)]
    pub non_interactive: bool,

    /// Set the logging verbosity level.
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Selected operational mode or isolated subsystem category.
    #[command(subcommand)]
    pub command: Commands,
}

macro_rules! define_commands {
    (
        SUB { $($sub_variant:ident($sub_action:ident) => $sub_desc:expr),* }
        FLAT { $($flat_variant:ident($flat_action:ident) => $flat_desc:expr),* }
        SIMPLE { $($simple_variant:ident => $simple_desc:expr),* }
    ) => {
        #[derive(Subcommand, Debug)]
        pub enum Commands {
            $(
                #[doc = $sub_desc]
                $sub_variant {
                    #[command(subcommand)]
                    action: $sub_action,
                },
            )*
            $(
                #[doc = $flat_desc]
                $flat_variant {
                    #[command(flatten)]
                    action: $flat_action,
                },
            )*
            $(
                #[doc = $simple_desc]
                $simple_variant,
            )*
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
                        Commands::$sub_variant { action } => action.execute().context(concat!(stringify!($sub_variant), " failure")),
                    )*
                    $(
                        Commands::$flat_variant { action } => action.execute().context(concat!(stringify!($flat_variant), " failure")),
                    )*
                    $(
                        Commands::$simple_variant => {
                            match stringify!($simple_variant) {
                                "CrashRecovery" => crate::commands::runtime::crash_recovery::execute().context("Crash recovery failure"),
                                _ => Ok(())
                            }
                        }
                    )*
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
    SUB {
        Build(BuildAction) => "Infrastructure build operations",
        Run(RunAction) => "Emulation and deployment gateways",
        Test(TestAction) => "Validation suites",
        Setup(SetupAction) => "Host setup and bootstrapping",
        Dashboard(DashboardAction) => "Pipeline health visualization",
        LinuxAbi(LinuxAbiAction) => "Linux ABI compatibility",
        Secureboot(SecurebootAction) => "Secure Boot protocols",
        Release(ReleaseAction) => "Release engineering",
        AbSlot(AbSlotAction) => "A/B slot management",
        Glibc(GlibcAction) => "Glibc audit",
        Pipeline(PipelineAction) => "Unified pipeline orchestrator",
        Completion(CompletionAction) => "Generate shell completion scripts",
        Interactive(InteractiveAction) => "Interactive build & distro management"
    }
    FLAT {
        Clean(CleanAction) => "Purge build artifacts, staging areas, and temporary files"
    }
    SIMPLE {
        CrashRecovery => "Panic diagnostics"
    }
}
