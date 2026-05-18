use crate::engine::{ExecutionContext, Task, TaskStatus};
use crate::utils::fs::paths::LAYOUT;
use crate::utils::logging;
use crate::utils::sys::process::Executor;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub struct Macro {
    pub name: String,
    pub commands: Vec<String>,
}

impl Task for Macro {
    fn name(&self) -> String {
        format!("Macro: {}", self.name)
    }
    fn description(&self) -> String {
        "Sequentially executes a set of pre-recorded build commands".to_string()
    }

    fn run(&self, _ctx: &ExecutionContext) -> Result<TaskStatus> {
        replay_macro(&self.name)?;
        Ok(TaskStatus::Success)
    }
}

pub fn record_macro(name: &str, commands: Vec<String>) -> Result<()> {
    let path = get_macro_path(name);
    let content = commands.join("\n");
    fs::write(path, content)?;
    logging::success(
        "MACRO",
        &format!("Macro '{}' recorded with {} commands", name, commands.len()),
        &[],
    );
    Ok(())
}

thread_local! {
    static MACRO_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub fn replay_macro(name: &str) -> Result<()> {
    let _depth = MACRO_DEPTH.with(|d| {
        let current = d.get();
        if current > 5 {
            return Err(anyhow::anyhow!("Max macro recursion depth reached (5)"));
        }
        d.set(current + 1);
        Ok(current + 1)
    })?;

    let path = get_macro_path(name);
    if !path.exists() {
        anyhow::bail!("Macro '{}' not found at {}", name, path.display());
    }

    let content = fs::read_to_string(path)?;
    let commands: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();

    logging::status("MACRO", &format!("Replaying macro: {}", name));
    for cmd in commands {
        let args = shlex::split(cmd).context("Failed to parse macro command string")?;
        Executor::new("cargo")
            .arg("xtask")
            .args(&args)
            .run()
            .with_context(|| format!("Macro failed while executing: {}", cmd))?;
    }

    Ok(())
}

fn get_macro_path(name: &str) -> PathBuf {
    LAYOUT
        .root
        .join(".xtask")
        .join("macros")
        .join(format!("{}.macro", name))
}
