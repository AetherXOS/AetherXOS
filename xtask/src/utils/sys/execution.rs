use super::sentinel::Sentinel;
use crate::utils::logging;
use anyhow::{Context, Result, bail};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Mutex;

pub static TUI_HUD_LOG_SENDER: Lazy<Mutex<Option<crossbeam_channel::Sender<String>>>> =
    Lazy::new(|| Mutex::new(None));

/// A fluent builder for executing processes with logging and tracking.
pub struct Executor {
    program: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    env: HashMap<String, String>,
    capture_output: bool,
    best_effort: bool,
    use_progress: bool,
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
            use_progress: true,
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

    pub fn with_progress(mut self) -> Self {
        self.use_progress = true;
        self
    }

    pub fn without_progress(mut self) -> Self {
        self.use_progress = false;
        self
    }

    pub fn run(self) -> Result<()> {
        let label = self.program.clone();
        let cmd_str = self.to_command_string();
        logging::exec(&label, &cmd_str);

        if self.use_progress {
            return self.run_with_progress();
        }

        let mut command = self.build_command();
        let mut child = command
            .spawn()
            .context(format!("Failed to spawn {}", self.program))?;

        Sentinel::track(&child);
        let status = child.wait()?;

        if !status.success() && !self.best_effort {
            bail!(
                "{} failed with exit code {}",
                self.program,
                status.code().unwrap_or(-1)
            );
        }
        Ok(())
    }

    fn run_with_progress(self) -> Result<()> {
        use indicatif::{ProgressBar, ProgressStyle};
        use std::io::{BufRead, BufReader};

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⼼⠴⠦⠧⠇⠏ "),
        );
        pb.set_message(format!("Starting {}...", self.program));

        let is_tui = crate::utils::core::config::get_settings().tui_hud_enabled;
        if is_tui {
            pb.set_draw_target(indicatif::ProgressDrawTarget::hidden());
        }

        let hud_sender = crate::utils::sys::execution::TUI_HUD_LOG_SENDER
            .lock()
            .unwrap()
            .clone();
        let hud_sender_clone = hud_sender.clone();

        let mut command = self.build_command();
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command
            .spawn()
            .context(format!("Failed to spawn {}", self.program))?;
        Sentinel::track(&child);

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let pb_clone1 = pb.clone();
        let stdout_thread = std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(l) = line {
                    if let Some(ref tx) = hud_sender {
                        let _ = tx.send(l.clone());
                    }
                    pb_clone1.set_message(l);
                }
            }
        });

        let hud_sender_clone2 = hud_sender_clone;
        let pb_clone2 = pb.clone();
        let stderr_thread = std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(l) = line {
                    if let Some(ref tx) = hud_sender_clone2 {
                        let _ = tx.send(l.clone());
                    }
                    pb_clone2.set_message(l);
                }
            }
        });

        let status = child.wait()?;
        let _ = stdout_thread.join();
        let _ = stderr_thread.join();

        if !status.success() && !self.best_effort {
            pb.finish_with_message(format!("❌ Failed: {}", self.program));
            bail!(
                "{} failed with exit code {}",
                self.program,
                status.code().unwrap_or(-1)
            );
        }

        pb.finish_and_clear();
        Ok(())
    }

    pub fn run_capture(self) -> Result<String> {
        let mut command = self.build_command();
        command.stdout(Stdio::piped());

        let output = command
            .output()
            .context(format!("Failed to execute {}", self.program))?;
        if !output.status.success() && !self.best_effort {
            bail!(
                "{} failed with exit code {}",
                self.program,
                output.status.code().unwrap_or(-1)
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn run_with_output(self) -> Result<(ExitStatus, String, String)> {
        let mut command = self.build_command();
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let output = command
            .output()
            .context(format!("Failed to execute {}", self.program))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok((output.status, stdout, stderr))
    }

    pub fn run_status(self) -> Result<ExitStatus> {
        let mut command = self.build_command();
        let mut child = command
            .spawn()
            .context(format!("Failed to spawn {}", self.program))?;
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
