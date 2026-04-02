use anyhow::{Result, bail};

use crate::cli::DashboardAction;
use crate::constants;
use crate::constants::{commands, npm};
use crate::utils::process;

/// Entry point for `cargo run -p xtask -- dashboard <action>`.
pub fn execute(action: &DashboardAction) -> Result<()> {
    let dashboard_dir = constants::paths::dashboard_dir();
    match action {
        DashboardAction::Build => {
            println!("[dashboard::build] Regenerating dashboard data, HTML, and UI assets");
            process::run_checked_in_dir(
                process::npm_bin(),
                &[npm::ARG_RUN, npm::SCRIPT_BUILD],
                &dashboard_dir,
            )
        }
        DashboardAction::Test => {
            println!("[dashboard::test] Running dashboard unit and E2E tests");
            process::run_checked_in_dir(
                process::npm_bin(),
                &[npm::ARG_RUN, npm::SCRIPT_CHECK],
                &dashboard_dir,
            )?;
            process::run_checked_in_dir(
                process::npm_bin(),
                &[
                    npm::ARG_RUN,
                    npm::SCRIPT_TEST_UNIT,
                    npm::ARG_SEPARATOR,
                    npm::ARG_TEST_RUN,
                ],
                &dashboard_dir,
            )
        }
        DashboardAction::Open => {
            println!("[dashboard::open] Opening dashboard in browser");
            let ui_index = dashboard_dir.join(npm::BUILD_OUTPUT_PATH);
            if !ui_index.exists() {
                process::run_checked_in_dir(
                    process::npm_bin(),
                    &[npm::ARG_RUN, npm::SCRIPT_BUILD],
                    &dashboard_dir,
                )?;
            }
            let target = ui_index.to_string_lossy().to_string();
            if cfg!(windows) {
                process::run_checked(
                    commands::windows::CMD_SHELL,
                    &[
                        commands::windows::CMD_FLAG,
                        commands::windows::CMD_START,
                        "",
                        &target,
                    ],
                )
            } else {
                process::run_checked(commands::unix::CMD_OPEN, &[&target])
            }
        }
        DashboardAction::AgentStart { no_safe } => {
            if *no_safe {
                println!("[dashboard::agent] Starting agent (unsafe/no-auth mode)");
                let status = process::run_status_in_dir(
                    process::npm_bin(),
                    &[
                        npm::ARG_RUN,
                        npm::SCRIPT_DEV,
                        npm::ARG_SEPARATOR,
                        npm::ARG_HOST,
                        npm::HOST_UNSAFE,
                    ],
                    &dashboard_dir,
                )?;
                if !status.success() {
                    bail!("dashboard agent dev server failed");
                }
                Ok(())
            } else {
                println!("[dashboard::agent] Starting agent in background");
                let status = process::run_status_in_dir(
                    process::npm_bin(),
                    &[
                        npm::ARG_RUN,
                        npm::SCRIPT_DEV,
                        npm::ARG_SEPARATOR,
                        npm::ARG_HOST,
                        npm::HOST_SAFE,
                    ],
                    &dashboard_dir,
                )?;
                if !status.success() {
                    bail!("dashboard agent dev server failed");
                }
                Ok(())
            }
        }
    }
}
