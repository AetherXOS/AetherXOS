use crate::utils::logging;
use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

pub struct CommandOptions<'a> {
    pub cwd: Option<&'a Path>,
    pub envs: Option<&'a [(&'a str, &'a str)]>,
}

impl<'a> CommandOptions<'a> {
    pub const fn new() -> Self {
        Self {
            cwd: None,
            envs: None,
        }
    }

    pub const fn cwd(mut self, cwd: &'a Path) -> Self {
        self.cwd = Some(cwd);
        self
    }

    pub const fn envs(mut self, envs: &'a [(&'a str, &'a str)]) -> Self {
        self.envs = Some(envs);
        self
    }
}

fn format_borrowed_args(args: &[&str]) -> String {
    args.join(" ")
}

fn format_owned_args(args: &[String]) -> String {
    args.join(" ")
}

fn ensure_success(
    program: &str,
    rendered: &str,
    cwd: Option<&Path>,
    status: std::process::ExitStatus,
) -> Result<()> {
    if !status.success() {
        let exit_code = status.code().unwrap_or(-1);
        if let Some(dir) = cwd {
            bail!(
                "command_failed: program={} exit_code={} cwd={} args={}",
                program,
                exit_code,
                dir.display(),
                rendered
            );
        }
        bail!(
            "command_failed: program={} exit_code={} args={}",
            program,
            exit_code,
            rendered
        );
    }
    Ok(())
}

fn run_with_borrowed_args(
    program: &str,
    args: &[&str],
    options: CommandOptions<'_>,
) -> Result<std::process::ExitStatus> {
    let rendered = format_borrowed_args(args);
    let cmdline = if let Some(dir) = options.cwd {
        format!("(cd {}) {} {}", dir.display(), program, rendered)
    } else {
        format!("{} {}", program, rendered)
    };
    logging::exec("exec", &cmdline);

    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(dir) = options.cwd {
        cmd.current_dir(dir);
    }
    if let Some(kv) = options.envs {
        cmd.envs(kv.iter().copied());
    }

    cmd.status().with_context(|| {
        if let Some(dir) = options.cwd {
            format!(
                "command_spawn_failed: program={} cwd={} args={}",
                program,
                dir.display(),
                rendered
            )
        } else {
            format!(
                "command_spawn_failed: program={} args={}",
                program, rendered
            )
        }
    })
}

fn run_with_owned_args(
    program: &str,
    args: &[String],
    options: CommandOptions<'_>,
) -> Result<std::process::ExitStatus> {
    let rendered = format_owned_args(args);
    logging::exec("exec", &format!("{} {}", program, rendered));

    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(dir) = options.cwd {
        cmd.current_dir(dir);
    }
    if let Some(kv) = options.envs {
        cmd.envs(kv.iter().copied());
    }

    cmd.status().with_context(|| {
        format!(
            "command_spawn_failed: program={} args={}",
            program, rendered
        )
    })
}

/// Return the platform-appropriate npm executable name.
pub fn npm_bin() -> &'static str {
    if cfg!(windows) { "npm.cmd" } else { "npm" }
}

/// Run an arbitrary command, only checking for success.
pub fn run_checked(program: &str, args: &[&str]) -> Result<()> {
    let rendered = format_borrowed_args(args);
    let status = run_with_borrowed_args(program, args, CommandOptions::new())?;
    ensure_success(program, &rendered, None, status)
}

#[allow(dead_code)]
pub fn run_checked_owned(program: &str, args: &[String]) -> Result<()> {
    let rendered = format_owned_args(args);
    let status = run_with_owned_args(program, args, CommandOptions::new())?;
    ensure_success(program, &rendered, None, status)
}

/// Run a command with explicit environment variables.
#[allow(dead_code)]
pub fn run_checked_with_env(program: &str, args: &[&str], envs: &[(&str, &str)]) -> Result<()> {
    let rendered = format_borrowed_args(args);
    let status = run_with_borrowed_args(program, args, CommandOptions::new().envs(envs))?;
    ensure_success(program, &rendered, None, status)
}

#[allow(dead_code)]
pub fn run_checked_with_env_owned(
    program: &str,
    args: &[String],
    envs: &[(&str, &str)],
) -> Result<()> {
    let rendered = format_owned_args(args);
    let status = run_with_owned_args(program, args, CommandOptions::new().envs(envs))?;
    ensure_success(program, &rendered, None, status)
}

/// Run an arbitrary command in a specific working directory.
#[allow(dead_code)]
pub fn run_checked_in_dir(program: &str, args: &[&str], cwd: &Path) -> Result<()> {
    let rendered = format_borrowed_args(args);
    let status = run_with_borrowed_args(program, args, CommandOptions::new().cwd(cwd))?;
    ensure_success(program, &rendered, Some(cwd), status)
}

/// Run a command and return whether it succeeded, without bubbling up launch failures.
pub fn run_best_effort(program: &str, args: &[&str]) -> bool {
    run_with_borrowed_args(program, args, CommandOptions::new())
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Run a command in a directory and return whether it succeeded, without bubbling up launch failures.
#[allow(dead_code)]
pub fn run_best_effort_in_dir(program: &str, args: &[&str], cwd: &Path) -> bool {
    run_with_borrowed_args(program, args, CommandOptions::new().cwd(cwd))
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Run a command in a specific directory and return raw exit status.
#[allow(dead_code)]
pub fn run_status_in_dir(
    program: &str,
    args: &[&str],
    cwd: &Path,
) -> Result<std::process::ExitStatus> {
    run_with_borrowed_args(program, args, CommandOptions::new().cwd(cwd))
}

/// Check if a binary is available on the system PATH.
pub fn which(binary: &str) -> bool {
    // On Windows, use `where`; on unix, use `which`.
    let probe = if cfg!(windows) { "where" } else { "which" };
    Command::new(probe)
        .arg(binary)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check whether any binary in the list exists on PATH.
pub fn which_any(binaries: &[&str]) -> bool {
    binaries.iter().copied().any(which)
}

/// Return the first binary from the list that exists on PATH.
pub fn first_available_binary<'a>(binaries: &'a [&'a str]) -> Option<&'a str> {
    binaries.iter().copied().find(|binary| which(binary))
}

pub fn find_qemu_system_x86_64() -> Option<String> {
    if let Some(binary) = first_available_binary(&["qemu-system-x86_64", "qemu-system-x86_64.exe"])
    {
        return Some(binary.to_string());
    }

    let program_files =
        std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
    let candidate = std::path::PathBuf::from(program_files)
        .join("qemu")
        .join("qemu-system-x86_64.exe");
    if candidate.exists() {
        return Some(candidate.to_string_lossy().to_string());
    }

    None
}

pub fn find_qemu_img() -> Option<&'static str> {
    first_available_binary(&["qemu-img", "qemu-img.exe"])
}

pub fn run_first_success(candidates: &[(&str, &[&str])]) -> bool {
    candidates
        .iter()
        .any(|(program, args)| run_best_effort(program, args))
}

pub fn wait_child_with_timeout(
    child: &mut std::process::Child,
    duration: std::time::Duration,
    poll_interval: std::time::Duration,
) -> std::io::Result<Option<std::process::ExitStatus>> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait()? {
            Some(status) => return Ok(Some(status)),
            None => {
                if start.elapsed() >= duration {
                    return Ok(None);
                }
                std::thread::sleep(poll_interval);
            }
        }
    }
}

pub fn read_optional_pipe_to_string<R: std::io::Read>(pipe: Option<R>) -> String {
    let mut buf = String::new();
    if let Some(mut reader) = pipe {
        let _ = std::io::Read::read_to_string(&mut reader, &mut buf);
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::{CommandOptions, first_available_binary, npm_bin, run_checked, run_checked_in_dir};
    use std::path::Path;

    fn nonzero_exit_command() -> (&'static str, Vec<&'static str>) {
        if cfg!(windows) {
            ("cmd", vec!["/C", "exit", "7"])
        } else {
            ("sh", vec!["-c", "exit 7"])
        }
    }

    fn nonzero_exit_snapshot_message() -> &'static str {
        if cfg!(windows) {
            "command_failed: program=cmd exit_code=7 args=/C exit 7"
        } else {
            "command_failed: program=sh exit_code=7 args=-c exit 7"
        }
    }

    #[test]
    fn command_options_default_is_empty() {
        let opts = CommandOptions::new();
        assert!(opts.cwd.is_none());
        assert!(opts.envs.is_none());
    }

    #[test]
    fn first_available_binary_returns_none_for_empty_list() {
        assert_eq!(first_available_binary(&[]), None);
    }

    #[test]
    fn npm_bin_is_platform_expected() {
        if cfg!(windows) {
            assert_eq!(npm_bin(), "npm.cmd");
        } else {
            assert_eq!(npm_bin(), "npm");
        }
    }

    #[test]
    fn run_checked_returns_command_failed_for_nonzero_exit() {
        let (program, args) = nonzero_exit_command();
        let err =
            run_checked(program, &args).expect_err("non-zero exit command must produce an error");
        let msg = err.to_string();
        assert_eq!(msg, nonzero_exit_snapshot_message());
    }

    #[test]
    fn run_checked_in_dir_includes_cwd_in_failure_message() {
        let (program, args) = nonzero_exit_command();
        let cwd = Path::new(".");
        let err = run_checked_in_dir(program, &args, cwd)
            .expect_err("non-zero exit command in dir must produce an error");
        let msg = err.to_string();
        assert!(msg.contains("command_failed:"));
        assert!(msg.contains("cwd="));
        assert!(msg.contains("exit_code=7"));
    }

    #[test]
    fn run_checked_returns_spawn_failure_for_missing_program() {
        let err = run_checked("definitely_missing_xtask_binary_123", &["--version"])
            .expect_err("missing program must produce spawn error");
        let msg = err.to_string();
        assert_eq!(
            msg,
            "command_spawn_failed: program=definitely_missing_xtask_binary_123 args=--version"
        );
    }
}
