#![allow(dead_code)]

mod builders;
mod cli;
mod commands;
mod config;
mod constants;
mod types;
mod utils;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use std::env;
use utils::{context as app_context, logging};

fn main() -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let cpu_model = get_cpu_model();
    let rustc_version = get_rustc_version();

    let about = format!("Aether X OS xtask v{} ({})", version, rustc_version);
    let system = format!("{} {} ({})", env::consts::OS, env::consts::ARCH, cpu_model);
    let target = format!(
        "AetherX - Mode: {}",
        if cfg!(debug_assertions) {
            "Development"
        } else {
            "Release"
        }
    );

    logging::print_header(&about, &system, &target);

    // Initial check for no args or explicit help
    let args_vec: Vec<String> = env::args().collect();
    if args_vec.len() == 1
        || args_vec
            .iter()
            .any(|a| a == "--help" || a == "-h" || a == "help")
    {
        utils::help::print_autonomous_help();
        if args_vec.len() == 1 || args_vec.contains(&"help".to_string()) {
            return Ok(());
        }
    }

    let args = Cli::parse();

    // If caller requested non-interactive via CLI flag, propagate to utils
    // config so helpers that consult `is_non_interactive()` see it.
    if args.non_interactive {
        utils::config::set_non_interactive(true);
    }

    utils::paths::ensure_dir(&args.outdir).context("Failed to initialize artifacts directory")?;
    app_context::init(args.outdir.clone()).context("Failed to initialize xtask runtime context")?;

    // Global Integrity and Pre-flight Audits
    utils::preflight::run_audit().context("System health audit encountered a terminal failure")?;

    // Autonomous execution via trait dispatch
    use utils::executable::Executable;
    args.command.execute()
}

fn get_cpu_model() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
            for line in content.lines() {
                if line.contains("model name") {
                    return line
                        .split(':')
                        .nth(1)
                        .unwrap_or("Unknown CPU")
                        .trim()
                        .to_string();
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
        {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("wmic")
            .args(["cpu", "get", "name"])
            .output()
        {
            let s = String::from_utf8_lossy(&output.stdout);
            return s
                .lines()
                .nth(1)
                .unwrap_or("Generic Windows CPU")
                .trim()
                .to_string();
        }
    }

    env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "Generic Host CPU".to_string())
}

fn get_rustc_version() -> String {
    std::process::Command::new("rustc")
        .arg("-V")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.split(' ').nth(1).unwrap_or("unknown").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
