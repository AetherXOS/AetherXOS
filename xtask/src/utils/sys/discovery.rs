use std::process::{Command, Stdio};
use anyhow::{Result, bail};
use crate::constants::tools;

/// Logic for discovering host-side binaries and tools.
pub struct Discovery;

impl Discovery {
    /// Check if a binary exists in PATH.
    pub fn which(cmd: &str) -> bool {
        let check_cmd = if cfg!(windows) { "where" } else { "which" };
        Command::new(check_cmd)
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Check if any of the given binaries exist.
    pub fn which_any(binaries: &[&str]) -> bool {
        binaries.iter().any(|&b| Self::which(b))
    }

    /// Returns the first available binary from a list.
    pub fn first_available<'a>(binaries: &[&'a str]) -> Option<&'a str> {
        binaries.iter().copied().find(|&b| Self::which(b))
    }

    /// Ensures a tool is available or returns an error.
    pub fn ensure_tool(name: &str) -> Result<()> {
        if Self::which(name) {
            Ok(())
        } else {
            bail!("Required tool '{}' not found in PATH", name)
        }
    }

    /// Find the platform-appropriate QEMU system binary.
    pub fn qemu_system_x86_64() -> Option<String> {
        let bin = if cfg!(windows) { tools::QEMU_X86_64_EXE } else { tools::QEMU_X86_64 };
        if Self::which(bin) { Some(bin.to_string()) } else { None }
    }

    /// Find the platform-appropriate qemu-img binary.
    pub fn qemu_img() -> Option<String> {
        let bin = if cfg!(windows) { tools::QEMU_IMG_EXE } else { tools::QEMU_IMG };
        if Self::which(bin) { Some(bin.to_string()) } else { None }
    }

    /// Return the correct npm binary name for the platform.
    pub const fn npm_bin() -> &'static str {
        if cfg!(windows) { "npm.cmd" } else { "npm" }
    }
}
