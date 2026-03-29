use anyhow::{Result, bail};

use crate::cli::DashboardAction;
use crate::utils::{process, paths};

/// Entry point for `cargo xtask dashboard <action>`.
pub fn execute(action: &DashboardAction) -> Result<()> {
    let dashboard_dir = paths::resolve("dashboard");
    match action {
        DashboardAction::Build => {
            println!("[dashboard::build] Regenerating dashboard data, HTML, and UI assets");
            process::run_checked_in_dir("npm", &["run", "build"], &dashboard_dir)
        }
        DashboardAction::Test => {
            println!("[dashboard::test] Running dashboard unit and E2E tests");
            process::run_checked_in_dir("npm", &["run", "check"], &dashboard_dir)?;
            process::run_checked_in_dir("npm", &["run", "test:unit", "--", "--run"], &dashboard_dir)
        }
        DashboardAction::Open => {
            println!("[dashboard::open] Opening dashboard in browser");
            let ui_index = dashboard_dir.join("dist/index.html");
            if !ui_index.exists() {
                process::run_checked_in_dir("npm", &["run", "build"], &dashboard_dir)?;
            }
            let target = ui_index.to_string_lossy().to_string();
            if cfg!(windows) {
                process::run_checked("cmd", &["/C", "start", "", &target])
            } else {
                process::run_checked("xdg-open", &[&target])
            }
        }
        DashboardAction::AgentStart { no_safe } => {
            if *no_safe {
                println!("[dashboard::agent] Starting agent (unsafe/no-auth mode)");
                let status = process::run_status_in_dir("npm", &["run", "dev", "--", "--host", "0.0.0.0"], &dashboard_dir)?;
                if !status.success() {
                    bail!("dashboard agent dev server failed");
                }
                Ok(())
            } else {
                println!("[dashboard::agent] Starting agent in background");
                let status = process::run_status_in_dir("npm", &["run", "dev", "--", "--host", "127.0.0.1"], &dashboard_dir)?;
                if !status.success() {
                    bail!("dashboard agent dev server failed");
                }
                Ok(())
            }
        }
    }
}
