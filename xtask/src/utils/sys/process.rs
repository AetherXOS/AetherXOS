//! Process utility facade.
//! DEPRECATED: Use Discovery, Sentinel, or Executor modules directly.

pub use super::discovery::Discovery;
pub use super::execution::{Executor, legacy};
pub use super::sentinel::Sentinel;

use anyhow::Result;
use std::path::Path;
use std::process::{Child, ExitStatus};
use std::time::Duration;

pub fn track_child(child: &Child) {
    Sentinel::track(child);
}
pub fn cleanup_all_children() {
    Sentinel::cleanup();
}
pub fn init_signal_handler() {
    Sentinel::init_signals();
}

pub fn run_with_output(cmd: &str, args: &[&str]) -> Result<(ExitStatus, String, String)> {
    Executor::new(cmd).args(args).run_with_output()
}

pub fn run_checked(cmd: &str, args: &[&str]) -> Result<()> {
    legacy::run_checked(cmd, args)
}

pub fn run_checked_owned(cmd: &str, args: &[String]) -> Result<()> {
    Executor::new(cmd).args(args).run()
}

pub fn run_checked_with_env_owned(cmd: &str, args: &[String], env: &[(&str, &str)]) -> Result<()> {
    let mut exec = Executor::new(cmd).args(args);
    for (k, v) in env {
        exec = exec.env(*k, *v);
    }
    exec.run()
}

pub fn run_checked_in_dir(cmd: &str, args: &[&str], dir: &Path) -> Result<()> {
    Executor::new(cmd).args(args).current_dir(dir).run()
}

pub fn run_status_in_dir(cmd: &str, args: &[&str], dir: &Path) -> Result<ExitStatus> {
    Executor::new(cmd).args(args).current_dir(dir).run_status()
}

pub fn run_status(cmd: &str, args: &[&str]) -> Result<ExitStatus> {
    legacy::run_status(cmd, args)
}

pub fn run_capture(cmd: &str, args: &[&str]) -> Result<String> {
    Executor::new(cmd).args(args).run_capture()
}

pub fn run_best_effort(cmd: &str, args: &[&str]) -> bool {
    Executor::new(cmd).args(args).best_effort().run().is_ok()
}

pub fn which(cmd: &str) -> bool {
    Discovery::which(cmd)
}
pub fn which_any(binaries: &[&str]) -> bool {
    Discovery::which_any(binaries)
}
pub fn ensure_tool(name: &str) -> Result<()> {
    Discovery::ensure_tool(name)
}
pub fn npm_bin() -> &'static str {
    Discovery::npm_bin()
}

pub fn find_qemu_system_x86_64() -> Option<String> {
    Discovery::qemu_system_x86_64()
}
pub fn find_qemu_img() -> Option<String> {
    Discovery::qemu_img()
}

pub fn is_git_dirty() -> bool {
    Executor::new("git")
        .args(&["status", "--porcelain"])
        .run_capture()
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

pub fn first_available_binary<'a>(binaries: &[&'a str]) -> Option<&'a str> {
    Discovery::first_available(binaries)
}

pub fn wait_child_with_timeout(
    child: &mut Child,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<Option<ExitStatus>> {
    let start = std::time::Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if start.elapsed() >= timeout {
            return Ok(None);
        }
        std::thread::sleep(poll_interval);
    }
}

pub fn read_optional_pipe_to_string<R: std::io::Read>(pipe: Option<R>) -> String {
    pipe.map(|mut p| {
        let mut s = String::new();
        let _ = p.read_to_string(&mut s);
        s
    })
    .unwrap_or_default()
}
