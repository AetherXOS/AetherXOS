mod cli;
mod commands;
mod config;
mod utils;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use std::env;
use utils::logging;

fn main() -> Result<()> {
    let about = format!("AetherXOS xtask/{} rustc/{}", env!("CARGO_PKG_VERSION"), "1.76.0");
    
    let cpu_info = if std::path::Path::new("/proc/cpuinfo").exists() {
        std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.contains("model name") || l.contains("Processor"))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|s| s.trim().to_string())
            })
            .unwrap_or_else(|| "Generic CPU".to_string())
    } else {
        "Host CPU".to_string()
    };

    let system = format!("{} {} ({})", env::consts::OS, env::consts::ARCH, cpu_info);
    let target = "x86_64-unknown-none (Release: false)";

    logging::print_header(&about, &system, target);

    let args = Cli::parse();
    let outdir = &args.outdir;
    
    utils::paths::ensure_dir(outdir)
        .with_context(|| format!("Failed to create global output directory context: {}", outdir.display()))?;

    env::set_var("XTASK_OUTDIR", outdir.to_str().unwrap_or("artifacts"));

    match args.command {
        cli::Commands::Build { ref action } => {
            commands::infra::build::execute(action)
                .context("Critical build pipeline failure")?;
        }
        cli::Commands::Run { ref action } => {
            commands::ops::run::execute(action)
                .context("Critical run operations pipeline failure")?;
        }
        cli::Commands::Test { ref action } => {
            commands::validation::test::execute(action)
                .context("Test suite execution failed")?;
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
            logging::info("main", "command branch undergoing modular refactoring", &[]);
        }
    }

    Ok(())
}
