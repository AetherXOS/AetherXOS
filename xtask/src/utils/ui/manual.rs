use crate::utils::logging;

pub fn show_manual() {
    logging::status("MANUAL", "AetherX XTask Operational Manual");
    println!("--------------------------------------------------");
    println!("COMMANDS:");
    println!("  pipeline run <name>    : Execute a named workflow");
    println!("  dashboard live         : Launch the TUI dashboard");
    println!("  completion generate    : Setup shell autocomplete");
    println!("  release draft          : Generate release notes");
    println!("--------------------------------------------------");
    println!("TIPS:");
    println!("  - Use --log-level debug for deep traces");
    println!("  - Profiles are saved in .xtask/profiles/");
    println!("--------------------------------------------------");
}
