use anyhow::{Result, Context};
use inquire::Select;
use colored::*;
use crate::utils::logging;
use super::menu_commands::*;

pub fn launch_main_menu() -> Result<()> {
    print_banner();
    
    let commands: Vec<Box<dyn MenuCommand>> = vec![
        Box::new(BuildKernelCommand),
        Box::new(DistroOpsCommand),
        Box::new(MacroOpsCommand),
        Box::new(ManageFeaturesCommand),
        Box::new(DashboardCommand),
        Box::new(SystemSettingsCommand),
        Box::new(ExitCommand),
    ];

    loop {
        print_system_info();
        
        let labels: Vec<&str> = commands.iter().map(|c| c.label()).collect();

        let selection = Select::new("AetherX Control Center", labels)
            .with_help_message("Use arrow keys to navigate, Enter to select")
            .prompt()
            .context("Failed to get menu selection")?;

        let cmd = commands.iter().find(|c| c.label() == selection).unwrap();
        
        match cmd.execute() {
            Ok(should_exit) => {
                if should_exit {
                    println!("{}", "\n  Thank you for using AetherX OS Build System. Happy coding! 🚀\n".green().bold());
                    break;
                }
            }
            Err(e) => {
                logging::error("INTERACTIVE", "Operation failed", &[("error", &format!("{:#}", e))]);
                if !crate::utils::ui::confirm("Return to main menu?", true).unwrap_or(false) {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn print_banner() {
    let banner = r#"
    █████╗ ███████╗████████╗██╗  ██╗███████╗██████╗ ██╗  ██╗
   ██╔══██╗██╔════╝╚══██╔══╝██║  ██║██╔════╝██╔══██╗╚██╗██╔╝
   ███████║█████╗     ██║   ███████║█████╗  ██████╔╝ ╚███╔╝ 
   ██╔══██║██╔══╝     ██║   ██╔══██║██╔══╝  ██╔══██╗ ██╔██╗ 
   ██║  ██║███████╗   ██║   ██║  ██║███████╗██║  ██║██╔╝ ██╗
   ╚═╝  ╚═╝╚══════╝   ╚═╝   ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝
    "#;
    println!("{}", banner.bright_magenta().bold());
    println!("{}", "   ░▒▓█ The Ultimate Agentic OS Build Engine █▓▒░".bright_cyan().bold());
    println!("{}", "   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());
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
