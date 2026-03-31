mod cli;
mod commands;
mod config;
mod utils;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use std::env;

fn main() -> Result<()> {
    // Parse command line arguments via clap interface.
    let args = Cli::parse();

    // Ensure the global output directory exists.
    let outdir = &args.outdir;
    utils::paths::ensure_dir(outdir)
        .with_context(|| format!("Failed to create global output directory context: {}", outdir.display()))?;

    // Export output directory explicitly to child processes or external commands
    // that rely on environmental context instead of direct argument passing.
    env::set_var("XTASK_OUTDIR", outdir.to_str().unwrap_or("artifacts"));

    // Route the requested subcommand to the appropriate isolated execution module.
    match args.command {
        cli::Commands::Build { ref action } => {
            commands::infra::build::execute(action)
                .context("Critical build pipeline failure")?;
        }
        cli::Commands::Run { ref action } => {
            commands::ops::run::execute(action)
                .context("Critical run operations pipeline failure")?;
        }
        cli::Commands::Setup { ref action } => {
            commands::infra::setup::execute(action)
                .context("Advanced toolchain host setup pipeline failed")?;
        }
        cli::Commands::LinuxAbi { ref action } => {
            commands::validation::linux_abi::execute(action)
                .context("Linux ABI subsystem gap parser failed unexpectedly")?;
        }
        cli::Commands::Secureboot { ref action } => {
            commands::infra::secureboot::execute(action)
                .context("UEFI Secure Boot validation protocol collapsed")?;
        }
        _ => {
            println!("[main] This command branch is currently deprecated or undergoing modular refactoring.");
        }
    }

    Ok(())
}
