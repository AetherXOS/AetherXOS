use anyhow::{bail, Context, Result};
use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, ExitStatus};

use crate::utils::logging;

pub fn run_checked(program: &str, args: &[&str]) -> Result<()> {
    run(CommandRequest::new(program).args(args))
}

pub fn run_checked_owned(program: &str, args: &[String]) -> Result<()> {
    run(CommandRequest::new(program).args(args))
}

pub fn run_checked_with_env_owned(
    program: &str,
    args: &[String],
    envs: &[(&str, &str)],
) -> Result<()> {
    run(CommandRequest::new(program).args(args).envs(envs))
}

pub fn run_checked_in_dir(program: &str, args: &[&str], cwd: &Path) -> Result<()> {
    run(CommandRequest::new(program).args(args).current_dir(cwd))
}

pub fn run_status_in_dir(program: &str, args: &[&str], cwd: &Path) -> Result<ExitStatus> {
    status(CommandRequest::new(program).args(args).current_dir(cwd))
}

pub fn which(binary: &str) -> bool {
    let probe = if cfg!(windows) { "where" } else { "which" };
    Command::new(probe)
        .arg(binary)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

struct CommandRequest<'a> {
    program: &'a str,
    args: Vec<&'a OsStr>,
    envs: Vec<(&'a str, &'a str)>,
    cwd: Option<&'a Path>,
}

impl<'a> CommandRequest<'a> {
    fn new(program: &'a str) -> Self {
        Self {
            program,
            args: Vec::new(),
            envs: Vec::new(),
            cwd: None,
        }
    }

    fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = &'a S>,
        S: AsRef<OsStr> + ?Sized + 'a,
    {
        self.args = args.into_iter().map(|arg| arg.as_ref().as_ref()).collect();
        self
    }

    fn envs(mut self, envs: &[(&'a str, &'a str)]) -> Self {
        self.envs = envs.to_vec();
        self
    }

    fn current_dir(mut self, cwd: &'a Path) -> Self {
        self.cwd = Some(cwd);
        self
    }

    fn render(&self) -> String {
        let command = if self.args.is_empty() {
            self.program.to_string()
        } else {
            format!(
                "{} {}",
                self.program,
                self.args
                    .iter()
                    .map(|arg| arg.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        };

        match self.cwd {
            Some(cwd) => format!("(cd {}) {}", cwd.display(), command),
            None => command,
        }
    }
}

fn run(request: CommandRequest<'_>) -> Result<()> {
    let rendered = request.render();
    logging::exec("exec", &rendered);

    let status = build_command(&request)
        .status()
        .with_context(|| format!("Failed to execute: {rendered}"))?;

    if !status.success() {
        bail!(
            "{} failed (exit code: {})",
            request.program,
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}

fn status(request: CommandRequest<'_>) -> Result<ExitStatus> {
    let rendered = request.render();
    logging::exec("exec", &rendered);

    build_command(&request)
        .status()
        .with_context(|| format!("Failed to execute: {rendered}"))
}

fn build_command(request: &CommandRequest<'_>) -> Command {
    let mut command = Command::new(request.program);
    command.args(&request.args);
    if let Some(cwd) = request.cwd {
        command.current_dir(cwd);
    }
    if !request.envs.is_empty() {
        command.envs(request.envs.iter().copied());
    }
    command
}
