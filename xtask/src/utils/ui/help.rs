use crate::cli::Cli;
use clap::CommandFactory;
use colored::*;

pub fn print_autonomous_help() {
    let cmd = Cli::command();

    println!("\n{}", " AETHER X OS ".on_bright_blue().white().bold());
    println!(
        "{}",
        format!(
            " Autonomous Pipeline Orchestrator v{}",
            env!("CARGO_PKG_VERSION")
        )
        .bright_blue()
        .italic()
    );

    println!("\n{}", "USAGE:".yellow().bold());
    println!("  xtask [OPTIONS] <COMMAND>");

    println!("\n{}", "COMMANDS:".yellow().bold());

    // Group subcommands by their doc comment categories or just list them
    for sub in cmd.get_subcommands() {
        let name = sub.get_name();
        let about = sub.get_about().map(|a| a.to_string()).unwrap_or_default();

        println!("  {: <18} {}", name.cyan().bold(), about);
    }

    if let Ok(registry) = std::fs::read_to_string(crate::utils::fs::paths::resolve(
        "xtask/distro-registry.json",
    )) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&registry) {
            if let Some(distros_obj) = json.get("distros").and_then(|v| v.as_object()) {
                println!(
                    "\n{}",
                    "REGISTERED DISTROS (Autonomous Discovery):"
                        .magenta()
                        .bold()
                );
                let distros: Vec<String> =
                    distros_obj.keys().take(8).map(|s| s.to_string()).collect();
                println!(
                    "  {} (and {} more...)",
                    distros.join(", ").cyan(),
                    distros_obj.len().saturating_sub(distros.len())
                );
            }
        }
    }

    println!("\n{}", "OPTIONS:".yellow().bold());
    for arg in cmd.get_arguments() {
        if arg.get_short().is_some() || arg.get_long().is_some() {
            let short = arg
                .get_short()
                .map(|s| format!("-{}, ", s))
                .unwrap_or_default();
            let long = arg
                .get_long()
                .map(|l| format!("--{}", l))
                .unwrap_or_default();
            let help = arg.get_help().map(|h| h.to_string()).unwrap_or_default();
            println!("  {: <18} {}", format!("{}{}", short, long).white(), help);
        }
    }

    println!(
        "\n{}",
        "Tip: Use 'xtask <COMMAND> --help' for deep inspection of any subsystem.".bright_black()
    );
}
