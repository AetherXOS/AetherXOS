use std::process::{Command, Stdio, ExitStatus};
use std::path::PathBuf;
use std::collections::HashMap;
use anyhow::{Result, Context, bail};
use crate::utils::logging;
use super::sentinel::Sentinel;

/// A fluent builder for executing processes with logging and tracking.
pub struct Executor {
    program: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    env: HashMap<String, String>,
    capture_output: bool,
    best_effort: bool,
}

impl Executor {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: HashMap::new(),
            capture_output: false,
            best_effort: false,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<S: AsRef<str>>(mut self, args: &[S]) -> Self {
        for arg in args {
            self.args.push(arg.as_ref().to_string());
        }
        self
    }

    pub fn current_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.cwd = Some(path.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn capture(mut self) -> Self {
        self.capture_output = true;
        self
    }

    pub fn best_effort(mut self) -> Self {
        self.best_effort = true;
        self
    }

    pub fn run(self) -> Result<()> {
        let label = self.program.clone();
        let cmd_str = self.to_command_string();
        logging::exec(&label, &cmd_str);

        let mut command = self.build_command();
        let mut child = command.spawn().context(format!("Failed to spawn {}", self.program))?;
        
        Sentinel::track(&child);
        let status = child.wait()?;

        if !status.success() && !self.best_effort {
            bail!("{} failed with exit code {}", self.program, status.code().unwrap_or(-1));
        }
        Ok(())
    }

    pub fn run_capture(self) -> Result<String> {
        let mut command = self.build_command();
        command.stdout(Stdio::piped());
        
        let output = command.output().context(format!("Failed to execute {}", self.program))?;
        if !output.status.success() && !self.best_effort {
            bail!("{} failed with exit code {}", self.program, output.status.code().unwrap_or(-1));
        }
        
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn run_with_output(self) -> Result<(ExitStatus, String, String)> {
        let mut command = self.build_command();
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        
        let output = command.output().context(format!("Failed to execute {}", self.program))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        
        Ok((output.status, stdout, stderr))
    }

    pub fn run_status(self) -> Result<ExitStatus> {
        let mut command = self.build_command();
        let mut child = command.spawn().context(format!("Failed to spawn {}", self.program))?;
        Sentinel::track(&child);
        Ok(child.wait()?)
    }

    fn build_command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        if let Some(ref cwd) = self.cwd {
            cmd.current_dir(cwd);
        }
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        cmd
    }

    fn to_command_string(&self) -> String {
        let mut s = String::new();
        if let Some(ref cwd) = self.cwd {
            s.push_str(&format!("(cd {}) ", cwd.display()));
        }
        s.push_str(&self.program);
        for arg in &self.args {
            s.push_str(&format!(" {}", arg));
        }
        s
    }
}

pub mod legacy {
    use super::*;

    pub fn run_checked(cmd: &str, args: &[&str]) -> Result<()> {
        Executor::new(cmd).args(args).run()
    }

    pub fn run_status(cmd: &str, args: &[&str]) -> Result<ExitStatus> {
        Executor::new(cmd).args(args).run_status()
    }
}
