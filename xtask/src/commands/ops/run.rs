use anyhow::Result;

use crate::cli::RunAction;

/// Entry point for `cargo xtask run <action>`.
pub fn execute(action: &RunAction) -> Result<()> {
    match action {
        RunAction::Smoke => {
            println!("[run::smoke] Starting automated kernel smoke test in QEMU");
            crate::commands::ops::qemu::smoke_test()?;
        }
        RunAction::Live => {
            println!("[run::live] Starting interactive QEMU session");
            crate::commands::ops::qemu::interactive()?;
        }
    }
    Ok(())
}
