use anyhow::{Result, Context};
use std::process::Command;
use crate::utils::logging;

pub struct Sandbox {
    pub name: String,
    pub use_wsl: bool,
}

impl Sandbox {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            use_wsl: cfg!(windows), // Default to WSL on Windows for isolation
        }
    }

    pub fn run_isolated(&self, command: &str, args: &[&str]) -> Result<()> {
        logging::status("SANDBOX", &format!("Running '{}' in isolated environment...", command));
        
        if self.use_wsl {
            // Translate command to WSL call
            let mut wsl_args = vec!["--exec", command];
            wsl_args.extend_from_slice(args);
            
            let status = Command::new("wsl")
                .args(&wsl_args)
                .status()
                .context("Failed to spawn WSL sandbox")?;
                
            if !status.success() {
                anyhow::bail!("Sandbox execution failed in WSL");
            }
        } else {
            // On Linux, we could use 'unshare' or 'systemd-run --wait --collect --pipe -p PrivateTmp=yes'
            let status = Command::new(command)
                .args(args)
                .status()
                .context("Failed to run command in local sandbox")?;
                
            if !status.success() {
                anyhow::bail!("Sandbox execution failed locally");
            }
        }
        
        Ok(())
    }
}
