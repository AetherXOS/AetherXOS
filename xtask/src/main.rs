mod cli;
mod commands;
mod config;
mod constants;
mod utils;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands};
use std::env;
use utils::logging;

fn main() -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let cpu_model = get_cpu_model();
    
    let about = format!("AetherXOS xtask/{} rustc/1.76.0", version);
    let system = format!("{} {} ({})", env::consts::OS, env::consts::ARCH, cpu_model);
    let target = "AetherXOS-Generic (Release: false)";

    logging::print_header(&about, &system, target);

    let args = Cli::parse();
    
    utils::paths::ensure_dir(&args.outdir)
        .with_context(|| format!("Failed to initialize artifacts directory: {}", args.outdir.display()))?;

    env::set_var("XTASK_OUTDIR", args.outdir.to_str().unwrap_or("artifacts"));

    match args.command {
        Commands::Build { ref action } => {
            commands::infra::build::execute(action).context("Build pipeline failure")?;
        }
        Commands::Run { ref action } => {
            commands::ops::run::execute(action).context("Execution pipeline failure")?;
        }
        Commands::Test { ref action } => {
            commands::validation::test::execute(action).context("Validation suite failure")?;
        }
        Commands::Setup { ref action } => {
            commands::infra::setup::execute(action).context("Host setup failure")?;
        }
        Commands::LinuxAbi { ref action } => {
            commands::validation::linux_abi::execute(action).context("ABI parser failure")?;
        }
        Commands::Secureboot { ref action } => {
            commands::infra::secureboot::execute(action).context("Secure Boot protocol failure")?;
        }
        Commands::Dashboard { ref action } => {
            commands::dashboard::execute(action).context("Dashboard operation failure")?;
        }
        Commands::Release { ref action } => {
            commands::release::preflight::execute(action).context("Release pipeline failure")?;
        }
        Commands::AbSlot { ref action } => {
            commands::runtime::ab_slot::execute(action).context("A/B slot operation failure")?;
        }
        Commands::CorePressure { ref words, ref lottery_words, ref format, ref out } => {
            commands::runtime::core_pressure::execute(words, lottery_words, format, out).context("Core pressure report failure")?;
        }
        Commands::CrashRecovery => {
            commands::runtime::crash_recovery::execute().context("Crash recovery pipeline failure")?;
        }
        Commands::Glibc { ref action } => {
            commands::validation::glibc::execute(action).context("Glibc audit failure")?;
        }
    }

    Ok(())
}

fn get_cpu_model() -> String {
    if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if line.contains("model name") || line.contains("Processor") {
                if let Some(model) = line.split(':').nth(1) {
                    return model.trim().to_string();
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sysctl").arg("-n").arg("machdep.cpu.brand_string").output() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    }

    env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "Generic Host CPU".to_string())
}
