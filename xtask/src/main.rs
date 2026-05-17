#![allow(dead_code)]

mod builders;
mod cli;
mod commands;
mod engine;
mod config;
mod constants;
mod types;
mod utils;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use std::env;
use utils::{context as app_context, logging};
use utils::fs::paths::LAYOUT;
use commands::interactive;

fn main() -> Result<()> {
    utils::sys::process::init_signal_handler();
    
    let version = env!("CARGO_PKG_VERSION");
    let about = format!("Aether X OS xtask v{} ({})", version, get_rustc_version());
    let system = format!("{} {} ({})", env::consts::OS, env::consts::ARCH, get_cpu_model());
    let target = format!("AetherX - Mode: {}", if cfg!(debug_assertions) { "Development" } else { "Release" });

    logging::print_header(&about, &system, &target);
    logging::init_logger(logging::LogLevel::Info, true)?;

    let args_vec: Vec<String> = env::args().collect();
    
    // 1. Interactive / Help Early Gates
    if args_vec.len() == 1 {
        LAYOUT.ensure_hermetic().context("Failed to stabilize project layout")?;
        app_context::init(LAYOUT.artifacts.clone()).context("Failed to initialize xtask runtime context")?;
        utils::preflight::NexusDoctor::new().audit(true).context("Nexus Doctor: System health audit failed")?;
        return interactive::menu::launch_main_menu();
    }

    if args_vec.len() == 2 && args_vec.iter().any(|a| a == "--help" || a == "-h" || a == "help") {
        let _ = utils::preflight::NexusDoctor::new().audit(false);
        crate::utils::ui::help::print_autonomous_help();
        return Ok(());
    }

    // 2. CLI Execution Flow
    let args = Cli::parse();
    
    // Configure environment
    if args.non_interactive {
        utils::config::set_non_interactive(true);
    }
    
    let log_lvl = match args.log_level.to_lowercase().as_str() {
        "trace" => logging::LogLevel::Trace,
        "debug" => logging::LogLevel::Debug,
        "warn"  => logging::LogLevel::Warn,
        "error" => logging::LogLevel::Error,
        _       => logging::LogLevel::Info,
    };
    logging::init_logger(log_lvl, true)?;

    // Stabilize Workspace
    LAYOUT.ensure_hermetic().context("Failed to stabilize project layout")?;
    app_context::init(args.outdir.clone()).context("Failed to initialize xtask runtime context")?;

    // Determine audit strictness
    let is_info_command = matches!(&args.command, cli::Commands::LinuxAbi { .. } | cli::Commands::Glibc { .. } | cli::Commands::Completion { .. });

    // 3. System Health Check
    let doctor = utils::preflight::NexusDoctor::new();
    doctor.audit(!is_info_command).context("Nexus Doctor: System health audit failed")?;

    // 4. Autonomous Command Execution
    use utils::executable::Executable;
    args.command.execute()
}

fn get_cpu_model() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
            for line in content.lines() {
                if line.contains("model name") {
                    return line.split(':').nth(1).unwrap_or("Unknown CPU").trim().to_string();
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("sysctl").args(["-n", "machdep.cpu.brand_string"]).output() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("wmic").args(["cpu", "get", "name"]).output() {
            let s = String::from_utf8_lossy(&output.stdout);
            return s.lines().nth(1).unwrap_or("Generic Windows CPU").trim().to_string();
        }
    }
    env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "Generic Host CPU".to_string())
}

fn get_rustc_version() -> String {
    std::process::Command::new("rustc").arg("-V").output().ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.split(' ').nth(1).unwrap_or("unknown").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
