use anyhow::{Result, Context};
use inquire::Select;
use colored::*;
use crate::utils::{logging, ui};
use crate::commands::interactive::features;
use crate::commands::interactive::config;

pub fn launch_main_menu() -> Result<()> {
    print_banner();
    
    loop {
        match run_menu_iteration() {
            Ok(should_exit) => {
                if should_exit {
                    println!("{}", "\n  Thank you for using AetherX OS Build System. Happy coding! 🚀\n".green().bold());
                    break;
                }
            }
            Err(e) => {
                logging::error("INTERACTIVE", "An error occurred during operation", &[
                    ("error", &format!("{:#}", e)),
                    ("context", "Please check the logs for details")
                ]);
                
                // Allow user to return to menu instead of crashing the whole xtask
                if !ui::confirm("Return to main menu?", true).unwrap_or(false) {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn run_menu_iteration() -> Result<bool> {
    print_system_info();
    
    let options = vec![
        "🚀 Build AetherX Kernel",
        "📦 Distro Operations (ISO, Rootfs)",
        "🛠️  Kernel Configuration (Features)",
        "🧪 Test Suite",
        "📊 Dashboard",
        "💾 Save/Load Configuration",
        "🚪 Exit"
    ];

    let selection = Select::new("AetherX Control Center", options)
        .with_help_message("Use arrow keys to navigate, Enter to select")
        .prompt()
        .context("Failed to get menu selection")?;

    match selection {
        "🚀 Build AetherX Kernel" => {
            crate::commands::infra::build::interactive::run()?;
            Ok(false)
        },
        "📦 Distro Operations (ISO, Rootfs)" => {
            logging::info("MENU", "Distro wizard is being initialized...", &[]);
            // We can add a standalone distro wizard here later
            Ok(false)
        },
        "🛠️  Kernel Configuration (Features)" => {
            features::manage_features()?;
            Ok(false)
        },
        "🧪 Test Suite" => {
            crate::commands::validation::test::run_interactive()?;
            Ok(false)
        },
        "📊 Dashboard" => {
            crate::commands::dashboard::execute()?;
            Ok(false)
        },
        "💾 Save/Load Configuration" => {
            config::manage_config()?;
            Ok(false)
        },
        "🚪 Exit" => Ok(true),
        _ => unreachable!(),
    }
}

fn print_banner() {
    let banner = r#"
    ___         __  __              _  __   ____  _____
   /   |  ___  / /_/ /_  ___  _____| |/ /  / __ \/ ___/
  / /| | / _ \/ __/ __ \/ _ \/ ___/|   /  / / / /\__ \ 
 / ___ |/  __/ /_/ / / /  __/ /   /   |  / /_/ /___/ / 
/_/  |_|\___/\__/_/ /_/\___/_/   /_/|_|  \____//____/  
    "#;
    println!("{}", banner.cyan().bold());
    println!("{}", "      --- The Ultimate Agentic OS Build Engine ---".dimmed());
    println!();
}

fn print_system_info() {
    let width = 58;
    let bar = "━".repeat(width).bright_black();
    println!("  {}", bar);
    println!("  {}  {} {}", "🕒".yellow(), "Time:".bold(), chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string().white());
    println!("  {}  {} {} {} ({})", "💻".blue(), "Host:".bold(), std::env::consts::OS, std::env::consts::ARCH, std::env::consts::FAMILY);
    println!("  {}", bar);
    println!();
}
