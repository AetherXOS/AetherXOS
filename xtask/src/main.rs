mod cli;
mod commands;
mod utils;
mod config;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        cli::Commands::Build { ref action } => {
            commands::infra::execute_build(action)?;
        }
        cli::Commands::Run { ref action } => {
            commands::ops::execute_run(action)?;
        }
        cli::Commands::Test { ref action } => {
            commands::validation::execute_test(action)?;
        }
        cli::Commands::Dashboard { ref action } => {
            commands::dashboard::execute(action)?;
        }
        cli::Commands::Setup { ref action } => {
            commands::infra::execute_setup(action)?;
        }
        cli::Commands::Secureboot { ref action } => {
            commands::validation::execute_secureboot(action)?;
        }
        cli::Commands::LinuxAbi { ref action } => {
            commands::validation::execute_linux_abi(action)?;
        }
        cli::Commands::Glibc { ref action } => {
            commands::validation::execute_glibc(action)?;
        }
        cli::Commands::Release { ref action } => {
            commands::release::execute_preflight(action)?;
        }
        cli::Commands::AbSlot { ref action } => {
            commands::runtime::execute_ab_slot(action)?;
        }
        cli::Commands::SyscallCoverage { linux_compat, ref format, ref out } => {
            commands::validation::execute_syscall_coverage(linux_compat, format, out)?;
        }
        cli::Commands::ArchiveNightly { ref run_id } => {
            commands::ops::execute_archive(run_id)?;
        }
        cli::Commands::SoakTest { dry_run } => {
            commands::ops::execute_soak(dry_run)?;
        }
        cli::Commands::CrashRecovery => {
            commands::runtime::execute_crash_recovery()?;
        }
        cli::Commands::CorePressure { ref words, ref lottery_replay_words, ref format, ref out } => {
            commands::runtime::execute_core_pressure(words, lottery_replay_words, format, out)?;
        }
        cli::Commands::TierStatus => {
            commands::release::execute_status()?;
        }
    }

    Ok(())
}
